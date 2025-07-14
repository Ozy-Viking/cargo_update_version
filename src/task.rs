use std::{
    collections::{HashMap, HashSet},
    process::{Child, ExitStatus, Output},
};

use semver::Version;
use tracing::{info, instrument};

use crate::{OutputExt, current_span};

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

    /// [Vec] of incomplete tasks.
    pub fn incomplete_tasks(&self) -> Vec<Task> {
        self.tasks
            .keys()
            .filter(|&k| !self.completed.contains(k))
            .cloned()
            .collect()
    }

    /// [Vec] of completed [Task].
    ///
    /// The underlying hashset can be accessed with [AsRef] and [AsMut]
    pub fn complete_tasks(&self) -> Vec<Task> {
        self.completed.iter().cloned().collect()
    }

    /// [bool] if remaining tasks exist based on [self.incomplete_tasks].
    pub fn remaining_tasks(&self) -> bool {
        !self.incomplete_tasks().is_empty()
    }

    pub fn remaining_tasks_left(&self) -> usize {
        self.incomplete_tasks().len()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Task {
    Push(String),
    Publish,
    Print,
    Set,
    Bump(BumpType),
    DeleteGitTag(Version),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum BumpType {
    Patch,
    Minor,
    Major,
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
    pub fn join_all(mut self) -> miette::Result<Vec<Task>, TaskError> {
        tracing::debug!("Starting to join tasks: {}", self.remaining_tasks_left());
        let span = current_span!();
        while self.remaining_tasks() {
            'tasks: for task in self.incomplete_tasks() {
                let child_option = match self.get_mut(&task) {
                    Some(c) => c,
                    None => {
                        span.record("remaining_tasks", self.remaining_tasks_left());
                        tracing::warn!("No child process existed for: {:?}", task);
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
                    tracing::warn!("No child process existed for: {:?}", task);
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
        assert_eq!(self.len(), self.complete_tasks().len());
        info!("All {} task/s complete!", self.complete_tasks().len());
        Ok(self.complete_tasks())
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
            completed_tasks: tasks.complete_tasks(),
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
