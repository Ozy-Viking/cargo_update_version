use std::{env::current_dir, fmt::Display, path::PathBuf};

use miette::{IntoDiagnostic, ensure, miette};

use crate::{Action, Branch, Bumpable, Cli, Packages, Result, Stash, Task, Tasks, VersionType};

pub struct DisplayTasks<'a> {
    tasks: &'a Tasks,
}

impl<'a> DisplayTasks<'a> {
    pub fn new(tasks: &'a Tasks) -> Self {
        Self { tasks: tasks }
    }

    pub fn display(&self) -> Result<()> {
        print!("{self}");
        Ok(())
    }

    fn task_item_string(&self, idx: usize, task: &'a Task) -> String {
        format!("├─ {idx}. {task}\n")
    }

    fn task_item_string_last(&self, idx: usize, task: &'a Task) -> String {
        format!("└─ {idx}. {task}\n")
    }

    pub fn tasks(&self) -> Vec<&Task> {
        self.tasks.tasks()
    }
}

impl Display for DisplayTasks<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut display = String::new();
        display.push_str(&format!("{} Tasks:\n", self.tasks.len()));
        let tasks = self.tasks();
        let last = tasks.last().expect("No way to have 0 tasks");
        for (idx, task) in tasks.iter().enumerate() {
            let s = if &task == &last {
                self.task_item_string_last(idx + 1, task)
            } else {
                self.task_item_string(idx + 1, task)
            };
            display.push_str(&s);
        }

        write!(f, "{display}")
    }
}

impl<'a> Tasks {
    #[allow(unused_variables)]
    /// Generate tasks from user defined [Cli] arguments.
    pub fn generate_tasks(cli_args: &'a Cli, packages: Packages) -> Result<Self> {
        cli_args.try_allow_dirty()?;
        let cwd = current_dir().into_diagnostic()?;
        let root_cargo_lock = packages.root_cargo_lock_path().to_path_buf();
        let root_manifest_path = packages.root_manifest_path().to_path_buf();
        let packages_clone = packages.clone();
        let mut tasks = Tasks::new(packages);
        let git = cli_args.git()?;
        let git_files = git.dirty_files()?;
        let cargo = cli_args.cargo()?;
        let workspace = cli_args.workspace();
        let pre_release = cli_args.pre();
        let build = cli_args.build();
        let force_version = cli_args.force_version();

        let current_branch = git.current_branch()?;
        let mut git_stash = None;

        let change_branch = if let Branch::Named { local } = cli_args.git_branch() {
            if !git_files.is_empty() {
                let git_stash_task = Task::GitStash {
                    branch: current_branch.clone(),
                    stash: Stash::Stash,
                };
                tasks.insert(git_stash_task.clone(), None);
                git_stash = Some(git_stash_task);
            }
            let c: Task = Task::GitSwitchBranch {
                to: local.into(),
                from: current_branch.clone(),
            };
            tasks.insert(c.clone(), None);
            Some(c)
        } else {
            None
        };

        let mut change_workspace_package_version: bool = cli_args.workspace_package(); // #40
        let mut paths_to_add: Vec<PathBuf> = Vec::new();
        let (included, excluded) = tasks.partition_packages_owned(workspace)?;
        ensure!(
            !included.is_empty(),
            help = "Check you are not excluding your root package without including others.",
            "No packages to modify. Excluded are: {:?}",
            excluded
                .iter()
                .map(|p| p.name().to_string())
                .collect::<Vec<_>>()
        );
        drop(excluded);
        for package in included {
            if package.version_type() == VersionType::SetByWorkspace {
                change_workspace_package_version = true;
                tracing::info!(
                    "Changing Workspace Package Version due to: {}",
                    package.name()
                );
            } else {
                paths_to_add.push(package.manifest_path_owned());

                // As the action needs to be applied to all included packages.
                let task = Task::from_action(
                    cli_args.action(),
                    &package,
                    cli_args.set_version(),
                    pre_release,
                    build,
                    force_version,
                )?;

                tasks.insert(task, None);
            }
        }

        if change_workspace_package_version {
            let mut new_version = tasks.packages_mut().workspace_package_mut().ok_or(miette!(help = "Is expected either from '--workspace-package' or a package's version is 'version.workspace = true'","Expected a workspace package, none was found."))?.version_owned();

            let task = match cli_args.action() {
                Action::Pre | Action::Patch | Action::Minor | Action::Major => {
                    new_version.bump(cli_args.action(), pre_release, build, force_version)?;
                    Some(Task::BumpWorkspace {
                        bump: cli_args.action(),
                        new_version,
                    })
                }
                Action::Set => Some(Task::SetWorkspace {
                    new_version: cli_args.set_version.clone().ok_or(miette::miette!(
                        "Expected a new version for Task::from_action when action is Set"
                    ))?,
                }),
                Action::Print | Action::Tree => None,
            };
            if let Some(t) = task {
                tasks.insert(t, None);
            }
        }

        let new_version = tasks.root_version()?;
        if cli_args.git_tag() {
            tasks.insert(Task::CargoGenerateLock, None);
            paths_to_add.push(root_cargo_lock);
            paths_to_add = paths_to_add
                .iter()
                .map(|p| match p.strip_prefix(&cwd) {
                    Ok(path) => path.to_path_buf(),
                    Err(_) => p.clone(),
                })
                .collect();
            tasks.insert(Task::GitAdd(paths_to_add), None);
            tasks.insert(Task::GitCommit, None);
            tasks.insert(Task::GitTag(new_version.clone()), None);
            if cli_args.git_push() {
                for remote in git.remotes()? {
                    tasks.insert(
                        Task::GitPush {
                            remote: remote,
                            branch: cli_args.git_branch(),
                            tag: new_version.to_string(),
                        },
                        None,
                    );
                }
            }
        }

        if cli_args.cargo_publish() {
            tasks.insert(Task::CargoPublish, None);
        }

        // 2nd Last
        if cli_args.dry_run() {
            tasks.insert(Task::DeleteGitTag(new_version.clone()), None);
        }

        // Last
        if let Some(Task::GitSwitchBranch { to, from }) = change_branch {
            tasks.insert(Task::GitSwitchBranch { to: from, from: to }, None);
        }

        if let Some(Task::GitStash {
            branch,
            stash: state,
        }) = git_stash
        {
            tasks.insert(
                Task::GitStash {
                    branch: branch,
                    stash: Stash::Unstash,
                },
                None,
            );
        }

        if cli_args.display_tasks() {
            DisplayTasks::new(&tasks).display()?;
        }

        Ok(tasks)
    }
}
