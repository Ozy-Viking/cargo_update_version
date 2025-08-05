use std::process::Child;

use indexmap::{IndexMap, IndexSet};

use semver::Version;
use tracing::{info, instrument};

use crate::{Cli, Package, PackageError, Packages, ReadToml, Result, cli::Workspace, current_span};

use super::{Task, TaskError};

#[allow(unused_imports)]
#[cfg(feature = "unstable")]
use std::str::FromStr;

#[derive(Debug)]
pub struct Tasks {
    tasks: IndexMap<Task, Option<Child>>,
    completed: IndexSet<Task>,
    packages: Packages,
}

impl Tasks {
    pub fn new(packages: Packages) -> Self {
        Self {
            packages,
            tasks: IndexMap::default(),
            completed: IndexSet::default(),
        }
    }

    #[instrument(skip(self))]
    pub fn append(&mut self, tasks: Vec<(Task, Option<Child>)>) {
        for (task, child) in tasks {
            tracing::debug!("Adding {task:?} to tasks");
            self.insert(task, child);
        }
    }

    /// Unordered [Vec] of all task keys as [Task].
    pub fn vec_keys(&self) -> Vec<Task> {
        self.keys().cloned().collect()
    }

    /// [`Vec<Task>`] of incomplete tasks.
    pub fn incomplete_tasks(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| !self.completed.contains(k))
            .cloned()
            .collect()
    }

    /// [`Vec<Task>`] of completed [Task].
    ///
    /// The underlying hashset can be accessed with [AsRef] and [AsMut]
    pub fn completed_tasks(&self) -> Vec<Task> {
        self.completed.iter().cloned().collect()
    }

    /// Adds the task to the Completed Hashset.
    ///
    /// Returns if the task is newly completed.
    pub fn complete_task(&mut self, task: &Task) -> bool {
        self.completed.insert(task.clone())
    }

    /// Collects a filtered [`Vec<Task>`] that should be completed after
    /// the main tasks using [Task::is_run_after_completed] method.
    pub fn run_after_completed_tasks(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| k.is_run_after_completed())
            .cloned()
            .collect()
    }

    /// [bool] if remaining tasks exist based on [self.incomplete_tasks].
    pub fn remaining_tasks(&self) -> bool {
        !self.incomplete_tasks().is_empty()
    }

    pub fn remaining_tasks_left(&self) -> usize {
        self.incomplete_tasks().len()
    }

    /// Collects a [`Vec<Task>`] of tasks that change a version of a package/s.
    pub fn version_change_tasks(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| !self.completed.contains(k) && k.is_version_change())
            .cloned()
            .collect()
    }

    /// Gets the [`DeleteGitTag`] [Task] from the [Tasks].
    ///
    /// [`DeleteGitTag`]: Task::DeleteGitTag
    pub fn get_delete_tag(&self) -> Option<&Task> {
        self.tasks.keys().find(|k| k.is_delete_git_tag())
    }

    pub fn tasks(&self) -> Vec<&Task> {
        self.tasks.keys().by_ref().collect()
    }
    pub fn tasks_owned(&self) -> Vec<Task> {
        self.tasks.keys().cloned().collect()
    }

    #[allow(dead_code)]
    #[cfg(feature = "unstable")]
    pub fn get_change_branch(&self) -> Option<&Task> {
        for task in self.tasks() {
            match task {
                Task::GitSwitchBranch { .. } => return Some(task),
                _ => continue,
            }
        }
        None
    }

    pub fn packages(&self) -> &Packages {
        &self.packages
    }

    pub fn packages_mut(&mut self) -> &mut Packages {
        &mut self.packages
    }

    pub fn set_packages(&mut self, packages: Packages) {
        self.packages = packages;
    }
    pub fn set_packages_mut(&mut self, packages: &mut Packages) {
        self.packages = packages.clone();
    }
}

impl std::ops::Deref for Tasks {
    type Target = IndexMap<Task, Option<Child>>;

    fn deref(&self) -> &Self::Target {
        &self.tasks
    }
}

impl std::ops::DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tasks
    }
}

impl AsRef<IndexMap<Task, Option<Child>>> for Tasks {
    fn as_ref(&self) -> &IndexMap<Task, Option<Child>> {
        &self.tasks
    }
}

impl AsMut<IndexMap<Task, Option<Child>>> for Tasks {
    fn as_mut(&mut self) -> &mut IndexMap<Task, Option<Child>> {
        &mut self.tasks
    }
}

impl AsRef<IndexSet<Task>> for Tasks {
    fn as_ref(&self) -> &IndexSet<Task> {
        &self.completed
    }
}

impl AsMut<IndexSet<Task>> for Tasks {
    fn as_mut(&mut self) -> &mut IndexSet<Task> {
        &mut self.completed
    }
}

impl Tasks {
    #[instrument(skip_all)]
    pub fn run_all(mut self, cli_args: &Cli) -> Result<Self> {
        tracing::debug!("Starting running tasks sequentially");
        let git = cli_args.git()?;
        let cargo = cli_args.cargo()?;
        let task_list = self.tasks_owned();
        let mut packages = self.packages.clone();

        for task in task_list {
            if task.is_run_after_completed() {
                continue;
            }
            match task.run(cli_args, &mut packages, &git, &cargo) {
                Ok(Some(c)) => {
                    let child = self
                        .get_mut(&task)
                        .expect("task should be present in tasks");
                    *child = Some(c)
                }
                Ok(None) => {
                    self.complete_task(&task);
                }
                Err(e) => {
                    tracing::error!("{task}, {e}");
                    return Err(TaskError::from_tasks(self, task, None, e.to_string()))?;
                }
            }
        }

        Ok(self)
    }

    #[allow(clippy::result_large_err)]
    #[instrument(skip_all, fields(remaining_tasks), name = "Tasks::join_all")]
    /// Joins all remaining [Task] with [Child] process.
    pub fn join_all(mut self) -> miette::Result<Tasks, TaskError> {
        tracing::debug!("Starting to join tasks: {}", self.remaining_tasks_left());
        let span = current_span!();
        while self.remaining_tasks() {
            'tasks: for task in self.incomplete_tasks() {
                let child_option = match self.get_mut(&task) {
                    Some(c) => c,
                    None => {
                        span.record("remaining_tasks", self.remaining_tasks_left());
                        tracing::info!("No child process existed for: {}", task);
                        self.completed.insert(task);
                        continue 'tasks;
                    }
                };

                let exit_status = if let Some(child) = child_option {
                    match child.try_wait() {
                        Ok(Some(exit_status)) => exit_status,
                        Ok(None) => {
                            // Task still going
                            continue 'tasks;
                        }
                        Err(e) => {
                            span.record("remaining_tasks", self.remaining_tasks_left());
                            let msg = format!("Error occured while running {task:?}: {}", e);
                            tracing::error!(msg);
                            return Err(TaskError::from_tasks(self, task, None, ""));
                        }
                    }
                } else {
                    span.record("remaining_tasks", self.remaining_tasks_left());
                    tracing::info!("No child process existed for: {}", task);
                    self.completed.insert(task);
                    continue 'tasks;
                };
                let output = child_option
                    .take()
                    .expect("Already contuned if none.")
                    .wait_with_output()
                    .expect("Already checked in try_wait.");

                if !exit_status.success() {
                    let msg = format!(
                        "{task:?} exited with code: {:?}",
                        output.status.code().unwrap_or_default()
                    );
                    span.record("remaining_tasks", self.remaining_tasks_left());
                    tracing::error!("{msg}");
                    return Err(TaskError::from_tasks(self, task, Some(output), msg));
                }
                self.completed.insert(task.clone());
                span.record("remaining_tasks", self.remaining_tasks_left());
                tracing::info!("{task:?} Complete");
            }
        }

        span.record("remaining_tasks", self.remaining_tasks_left());
        assert!(
            self.run_after_completed_tasks().len() == self.incomplete_tasks().len(),
            "Tasks is not equal to completed tasks"
        );
        info!("All {} task/s complete!", self.completed_tasks().len());
        Ok(self)
    }

    #[instrument(skip_all, fields(cleanup_tasks))]
    pub fn run_cleanup_tasks(self, cli_args: &Cli) -> Result<Self> {
        tracing::debug!("Starting running cleanup tasks");
        let git = cli_args.git()?;
        let cargo = cli_args.cargo()?;
        let task_list = self.run_after_completed_tasks();
        current_span!().record("cleanup_tasks", format!("{:?}", &task_list));
        tracing::trace!("cleanup tasks");
        let mut packages = self.packages.clone();
        for task in task_list {
            task.run(cli_args, &mut packages, &git, &cargo)?;
        }
        Ok(self)
    }
}

impl Tasks {
    pub fn partition_packages(
        &self,
        workspace: &Workspace,
    ) -> Result<(Vec<&Package<ReadToml>>, Vec<&Package<ReadToml>>)> {
        workspace.partition_packages(self.packages())
    }
    pub fn partition_packages_mut(
        &mut self,
        workspace: &Workspace,
    ) -> Result<(Vec<&mut Package<ReadToml>>, Vec<&mut Package<ReadToml>>)> {
        workspace.partition_packages_mut(self.packages_mut())
    }
    pub fn partition_packages_owned(
        &self,
        workspace: &Workspace,
    ) -> Result<(Vec<Package<ReadToml>>, Vec<Package<ReadToml>>)> {
        workspace.partition_packages_owned(self.packages())
    }
    /// Clones tasks but without any associated [`Child`] processes.
    pub fn clone_tasks(&self) -> Tasks {
        let tasks: Vec<(Task, Option<Child>)> = self.keys().cloned().map(|t| (t, None)).collect();
        Tasks {
            tasks: IndexMap::from_iter(tasks.into_iter()),
            completed: self.completed.clone(),
            packages: self.packages.clone(),
        }
    }

    /// Order of presedence:
    /// 1. Root package version
    /// 2. `workspace.package` version
    /// 3. ws members sharing the same version ... version
    pub fn root_version(&self) -> Result<Version> {
        let packages_root_version = self.packages.root_version()?;
        let version_tasks = self.version_change_tasks();
        let root_package = self.packages.get_root_package();

        let root_package_name = root_package.map(|p| p.name().clone()).unwrap_or_default();

        if version_tasks.is_empty() {
            return Ok(packages_root_version);
        }
        let mut versions = IndexSet::new();
        for task in version_tasks {
            match task {
                Task::Set {
                    package_name,
                    new_version,
                } => {
                    if package_name == root_package_name {
                        return Ok(new_version);
                    }
                    versions.insert((3, new_version));
                }
                Task::SetWorkspace { new_version } => {
                    versions.insert((2, new_version));
                }
                Task::Bump {
                    package_name,
                    new_version,
                    ..
                } => {
                    if package_name == root_package_name {
                        return Ok(new_version);
                    }
                    versions.insert((3, new_version));
                }
                Task::BumpWorkspace { new_version, .. } => {
                    versions.insert((2, new_version));
                }
                _ => unreachable!(),
            }
        }

        versions.sort_by(|a, b| a.0.cmp(&b.0));

        for (val, version) in &versions {
            if *val == 2 {
                return Ok(version.clone());
            }
        }
        if versions.len() == 1 {
            Ok(versions.pop().expect("Length of 1").1)
        } else {
            Err(PackageError::NoRootVersion)?
        }
    }
}

// TODO: Add tests to tasks.

#[cfg(test)]
mod tests {

    use super::*;

    #[cfg(feature = "unstable")]
    use crate::Branch;
    use crate::{Action, Bumpable, Cli, Packages};

    static TEST_BIN_NAME: &str = "cargo-uv";

    fn default_cli(manifest_path: &str) -> Cli {
        let args = vec![TEST_BIN_NAME, "--manifest-path", manifest_path]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        Cli::cli_args(args, Some(TEST_BIN_NAME), None).expect("Valid for testing")
    }

    fn simple_packages() -> Packages {
        let mut cli_args = default_cli("tests/fixtures/simple/Cargo.toml");
        let meta = cli_args
            .get_metadata()
            .expect("testing simple: tests/fixtures/simple/Cargo.toml");
        Packages::from(meta)
    }

    fn task_list<'a>(mut packages: Packages) -> Vec<Task> {
        let package = packages
            .get_root_package_mut()
            .expect("known that simple has a root package");
        let new_version = package
            .version_mut()
            .bump(Action::Major, None, None, false)
            .expect("Set by hand");
        vec![
            Task::Bump {
                package_name: package.name().clone(),
                bump: crate::Action::Major,
                new_version,
            },
            Task::GitPush {
                remote: "origin".into(),
                #[cfg(feature = "unstable")]
                branch: Branch::from_str("main").unwrap(),
                tag: package.version().to_string(),
            },
            Task::CargoPublish,
        ]
    }

    #[test]
    fn maintain_insertion_order_indexset() {
        let packages = simple_packages();
        let mut insertion_order = task_list(packages.clone());
        let mut tasks = Tasks::new(packages.clone());
        for task in &insertion_order {
            tasks.insert(task.clone(), None);
        }
        assert_eq!(
            insertion_order.iter().by_ref().collect::<Vec<_>>(),
            tasks.tasks()
        );
        insertion_order.reverse();
        assert_ne!(
            insertion_order.iter().by_ref().collect::<Vec<_>>(),
            tasks.tasks()
        )
    }
}
