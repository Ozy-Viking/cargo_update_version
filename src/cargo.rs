use std::process::{Child, Command};

use miette::IntoDiagnostic;

use crate::{cli::Cli, git::Git};

pub struct Cargo;
impl Cargo {
    pub fn command() -> Command {
        Command::new("cargo")
    }

    pub fn publish(cli_args: &Cli) -> miette::Result<Child> {
        let mut cargo = Cargo::command();
        cargo.arg("publish");
        if cli_args.dry_run() {
            cargo.arg("--dry-run");
        }
        Git::dirty_files()?;
        // TODO: Add no-verify to flags.
        // TODO: Be able to remove --allow-dirty
        cargo.args([
            "--color",
            "never",
            "--no-verify",
            "--quiet",
            "--allow-dirty",
        ]);

        cargo.spawn().into_diagnostic()
    }
}
