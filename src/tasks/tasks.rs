use std::{
    collections::{HashMap, HashSet},
    process::Child,
};

use tracing::{info, instrument};

use crate::current_span;

use super::{Task, TaskError};

// TODO: Add tests to tasks.
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

    /// Collects a filtered Vec of tasks that should have been completed before any clean up tasks.
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
            self.all_tasks_but_delete_tag().len() == self.completed_tasks().len(),
            "Tasks is not equal to completed tasks"
        );
        info!("All {} task/s complete!", self.completed_tasks().len());
        Ok(self)
    }
}
