mod predict_tasks;
pub use predict_tasks::DisplayTasks;
mod tasks;
pub use tasks::Tasks;
mod task;
pub use task::Task;

use std::process::{ExitStatus, Output};

use crate::OutputExt;

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
