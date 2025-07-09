use std::process::{Child, Command};

use miette::IntoDiagnostic;

use crate::{GitBuilder, cli::Cli};

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
        let git = GitBuilder::new()
            .root_directory(cli_args.root_dir()?)
            .build();
        git.dirty_files()?;
        // TODO: Add no-verify to flags.
        // TODO: Be able to remove --allow-dirty
        if cli_args.manifest.manifest_path.is_some() {
            cargo
                .arg("--manifest-path")
                .arg(cli_args.manifest.manifest_path.clone().unwrap());
        }
        if cli_args.no_verify() {
            cargo.arg("--no-verify");
        }

        if cli_args.dry_run() {
            cargo.arg("--dry-run");
        }
        cargo.args(["--color", "never", "--quiet", "--allow-dirty"]);

        cargo.spawn().into_diagnostic()
    }
}
