use miette::ensure;

use crate::{Action, Branch, Cli, Packages, Result, Task, Tasks, VersionType, cli};

pub struct DisplayTasks<'a> {
    cli: &'a Cli,
}

impl<'a> DisplayTasks<'a> {
    pub fn new(cli: &'a Cli) -> Self {
        Self { cli }
    }

    pub fn display(&self) -> Result<()> {
        Ok(())
    }
}

impl Tasks {
    /// Generate tasks from user defined [Cli] arguments.
    pub fn generate_tasks(mut cli_args: Cli) -> Result<Self> {
        let packages = Packages::from(cli_args.get_metadata()?);
        let mut tasks = Tasks::new();
        let git = cli_args.git()?;
        let cargo = cli_args.cargo()?;
        let mut change_workspace_package_version: bool = cli_args.workspace_package(); // #40
        let workspace = cli_args.workspace();
        let (included, excluded) = workspace.partition_packages(&packages)?;

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
                // As the action needs to be applied to all included packages.
                let task = Task::from_action(
                    cli_args.action(),
                    package.clone(),
                    cli_args.pre.clone(),
                    cli_args.build.clone(),
                    cli_args.set_version.clone(),
                    cli_args.force_version,
                )?;

                tasks.insert(task, None);
            }
        }

        if change_workspace_package_version {
            let task = match cli_args.action() {
                Action::Pre | Action::Patch | Action::Minor | Action::Major => {
                    Some(Task::BumpWorkspace {
                        bump: cli_args.action(),
                        pre: cli_args.pre.clone(),
                        build: cli_args.build.clone(),
                        force: cli_args.force_version,
                    })
                }
                Action::Set => Some(Task::SetWorkspace {
                    version: cli_args.set_version.clone().ok_or(miette::miette!(
                        "Expected a new version for Task::from_action when action is Set"
                    ))?,
                }),
                Action::Print | Action::Tree => None,
            };
            if let Some(t) = task {
                tasks.insert(t, None);
            }
        }

        // Return is assumed
        if let Branch::Other { local } = cli_args.git_branch() {
            tasks.insert(
                Task::ChangeBranch {
                    to: local,
                    from: git.current_branch()?.to_string(),
                },
                None,
            );
        };
        Ok(tasks)
    }
}
