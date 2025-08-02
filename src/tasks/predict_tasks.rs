use crate::{Cli, Result, Tasks};

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
    pub fn generate_tasks(cli_args: Cli) -> Result<Self> {
        let tasks = Tasks::new();

        Ok(tasks)
    }
}
