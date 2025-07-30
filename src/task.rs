use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    process::{Child, ExitStatus, Output},
};

use semver::{BuildMetadata, Prerelease, Version};
use tracing::{info, instrument};

use crate::{Action, OutputExt, Package, ReadToml, Result, current_span};

#[derive(Debug, Default)]
pub struct Tasks {
    tasks: HashMap<Task, Option<Child>>,
    completed: HashSet<Task>,
}

impl Tasks {
    pub fn new() -> Self {
        Self::default()
    }

    #[instrument(skip(self))]
    pub fn append(&mut self, tasks: Vec<(Task, Child)>) {
        for (task, child) in tasks {
            tracing::debug!("Adding {task:?} to tasks");
            self.insert(task, Some(child));
        }
    }

    /// Unordered [Vec] of all task keys as [Task].
    pub fn vec_keys(&self) -> Vec<Task> {
        self.keys().cloned().collect()
    }

    /// [Vec<Task>] of incomplete tasks.
    pub fn incomplete_tasks(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| !self.completed.contains(k))
            .cloned()
            .collect()
    }

    /// [Vec<Task>] of completed [Task].
    ///
    /// The underlying hashset can be accessed with [AsRef] and [AsMut]
    pub fn completed_tasks(&self) -> Vec<Task> {
        self.completed.iter().cloned().collect()
    }

    pub fn complete_task(&mut self, task: &Task) -> bool {
        self.completed.insert(task.clone())
    }

    pub fn all_tasks_but_delete_tag(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| !k.is_delete_git_tag())
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

    pub fn version_change_tasks(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| !self.completed.contains(k) && k.is_version_change())
            .cloned()
            .collect()
    }

    pub fn delete_tag(&self) -> Option<&Task> {
        self.tasks.keys().find(|k| k.is_delete_git_tag())
    }
}

#[derive(Hash, PartialEq, Debug, Eq, Clone)]
pub enum Task {
    Push(String),
    Publish,
    Print,
    Tree,
    Set {
        version: Version,
        package: Package<ReadToml>,
    },
    Bump {
        package: Package<ReadToml>,
        bump: Action,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force: bool,
    },
    BumpWorkspace {
        bump: Action,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force: bool,
    },
    SetWorkspace {
        version: Version,
    },
    DeleteGitTag(Version),
    ChangeBranch {
        to: String,
        from: String,
    },
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Task::Push(remote) => &format!("Push to {remote}"),
            Task::Publish => "Publish",
            Task::Print => "Print",
            Task::Tree => "Tree",
            Task::Set { version, package } => {
                &format!("Set {}: {}", package.name(), version.to_string())
            }
            Task::Bump { package, bump, .. } => &format!("Bump {bump}: {}", package.name()),
            Task::BumpWorkspace { bump, .. } => &format!("Bump Workspace Package: {}", bump),
            Task::SetWorkspace { version } => &format!("Set Workspace: {}", version.to_string()),
            Task::DeleteGitTag(version) => &format!("Delete Git Tag: {}", version.to_string()),
            Task::ChangeBranch { to, .. } => &format!("Change branch: {}", to),
        };
        write!(f, "{}", text)
    }
}

impl Task {
    pub fn is_version_change(&self) -> bool {
        match self {
            Task::ChangeBranch { .. }
            | Task::Push(_)
            | Task::Publish
            | Task::Print
            | Task::DeleteGitTag(_)
            | Task::Tree => false,

            Task::Set { .. }
            | Task::Bump { .. }
            | Task::BumpWorkspace { .. }
            | Task::SetWorkspace { .. } => true,
        }
    }

    /// Returns `true` if the task is [`DeleteGitTag`].
    ///
    /// [`DeleteGitTag`]: Task::DeleteGitTag
    #[must_use]
    pub fn is_delete_git_tag(&self) -> bool {
        matches!(self, Self::DeleteGitTag(..))
    }
}

/// TODO: Make a reference.
impl Task {
    pub fn from_action(
        action: Action,
        package: Package<ReadToml>,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        new_version: Option<Version>,
        force: bool,
    ) -> Result<Task> {
        match action {
            Action::Pre | Action::Patch | Action::Minor | Action::Major => Ok(Task::Bump {
                package: package,
                bump: action,
                pre,
                build,
                force,
            }),
            Action::Set => Ok(Task::Set {
                version: new_version.ok_or(miette::miette!(
                    "Expected a new version for Task::from_action when action is Set"
                ))?,
                package,
            }),
            Action::Print => Ok(Task::Tree),
            Action::Tree => Ok(Task::Print),
        }
    }
}

impl Task {
    pub fn run(&mut self) -> Option<Child> {
        None
    }
}

impl std::ops::Deref for Tasks {
    type Target = HashMap<Task, Option<Child>>;

    fn deref(&self) -> &Self::Target {
        &self.tasks
    }
}

impl std::ops::DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tasks
    }
}

impl AsRef<HashMap<Task, Option<Child>>> for Tasks {
    fn as_ref(&self) -> &HashMap<Task, Option<Child>> {
        &self.tasks
    }
}

impl AsMut<HashMap<Task, Option<Child>>> for Tasks {
    fn as_mut(&mut self) -> &mut HashMap<Task, Option<Child>> {
        &mut self.tasks
    }
}

impl AsRef<HashSet<Task>> for Tasks {
    fn as_ref(&self) -> &HashSet<Task> {
        &self.completed
    }
}

impl AsMut<HashSet<Task>> for Tasks {
    fn as_mut(&mut self) -> &mut HashSet<Task> {
        &mut self.completed
    }
}

impl Tasks {
    #[allow(clippy::result_large_err)]
    #[instrument(skip_all, fields(remaining_tasks), name = "Tasks::join_all")]
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
            self.all_tasks_but_delete_tag().len() == self.completed_tasks().len(),
            "Tasks is not equal to completed tasks"
        );
        info!("All {} task/s complete!", self.completed_tasks().len());
        Ok(self)
    }
}

impl TaskError {
    pub fn from_tasks(
        tasks: Tasks,
        errored_task: Task,
        output: Option<Output>,
        msg: impl Into<String>,
    ) -> Self {
        Self {
            completed_tasks: tasks.completed_tasks(),
            incomplete_tasks: tasks.incomplete_tasks(),
            errored_task,
            output: output
                .as_ref()
                .map(|o| o.stderr())
                .unwrap_or("Unknown Ourput".into()),
            status_code: output.as_ref().map(|o| o.status),
            msg: msg.into(),
        }
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{output}")]
#[diagnostic(code(TaskError))]
pub struct TaskError {
    pub completed_tasks: Vec<Task>,
    pub incomplete_tasks: Vec<Task>,
    pub errored_task: Task,
    pub output: String,
    pub status_code: Option<ExitStatus>,
    #[help]
    pub msg: String,
}

// TODO: Add tests to tasks.
