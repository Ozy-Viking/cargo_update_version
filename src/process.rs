use miette::{IntoDiagnostic, bail};
use tracing::instrument;

use crate::{Result, current_span};
use std::process::{Child, Command, Output};

pub trait OutputExt {
    fn stderr(&self) -> String;
    fn stdout(&self) -> String;
}

impl OutputExt for Output {
    fn stderr(&self) -> String {
        String::from_iter(self.stderr.iter().map(|&c| char::from(c)))
    }

    fn stdout(&self) -> String {
        String::from_iter(self.stdout.iter().map(|&c| char::from(c)))
    }
}

// TODO: #41 Create an enum for spawn and output to run the process and display debug info pre running.

#[derive(Debug)]
pub enum ProcessOutput {
    Output(Output),
    Child(Child),
}

impl ProcessOutput {
    #[track_caller]
    pub fn as_output(&self) -> Option<&Output> {
        if let Self::Output(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[track_caller]
    pub fn as_child(&self) -> Option<&Child> {
        if let Self::Child(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Errors if not [`ProcessOutput::Child`]
    #[track_caller]
    pub fn try_into_child(self) -> Result<Child> {
        if let Self::Child(v) = self {
            Ok(v)
        } else {
            bail!(
                help = format!("Called from: {}", std::panic::Location::caller()),
                "Expected to be child"
            )
        }
    }

    /// Errors if not [`ProcessOutput::Output`]
    #[track_caller]
    pub fn try_into_output(self) -> Result<Output> {
        if let Self::Output(v) = self {
            Ok(v)
        } else {
            bail!(
                help = format!("Called from: {}", std::panic::Location::caller()),
                "Expected to be output"
            )
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Process {
    Output,
    Spawn,
}

impl Process {
    /// Run commands with the correct output and debugging.
    #[track_caller]
    #[instrument(skip(cmd), name = "Process::run", fields(program))]
    pub fn run(&self, mut cmd: Command) -> Result<ProcessOutput> {
        let span = current_span!();
        span.record("program", cmd.get_program().to_str().unwrap_or_default());

        tracing::debug!("Running: {}", Process::display_command(&cmd));
        match self {
            Process::Output => Ok(ProcessOutput::Output(cmd.output().into_diagnostic()?)),
            Process::Spawn => Ok(ProcessOutput::Child(cmd.spawn().into_diagnostic()?)),
        }
    }

    /// Turns a [Command] into a [String] for displaying.
    ///
    /// ```no_run
    /// let mut cmd = Command::new("git");
    /// cmd.arg("not").arg("a").arg("command");
    /// assert_eq!("git not a command", Process::display_command(&cmd).as_str());
    /// ```
    pub fn display_command(cmd: &Command) -> String {
        let program = cmd.get_program();
        let args = cmd
            .get_args()
            .map(|a| a.to_str().unwrap_or_default())
            .collect::<Vec<_>>();
        let mut program = program.to_str().unwrap_or_default().to_string() + " ";
        let args = &args.join(" ");
        program.push_str(args);
        program.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_command_correctly_using_arg() {
        let mut cmd = Command::new("git");
        cmd.arg("not").arg("a").arg("command");
        assert_eq!("git not a command", Process::display_command(&cmd).as_str());
    }

    #[test]
    fn display_command_correctly_using_args() {
        let mut cmd = Command::new("git");
        cmd.args(["not", "a", "command"]);
        assert_eq!("git not a command", Process::display_command(&cmd).as_str());
    }
}
