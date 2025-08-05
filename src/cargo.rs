use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use miette::IntoDiagnostic;
use tracing::{debug, instrument};

use crate::{GitBuilder, cli::Cli};

#[derive(Debug, Default)]
pub struct Cargo {
    manifest_path: Option<PathBuf>,
}
impl Cargo {
    pub fn new(manifest_path: Option<PathBuf>) -> Self {
        Self { manifest_path }
    }

    #[instrument(name = "Cargo::command")]
    pub fn command(&self, supress_stdout: bool) -> Command {
        let mut cargo = Command::new("cargo");
        if !supress_stdout {
            debug!("Inherit");
            cargo.stdout(Stdio::inherit());
        } else {
            cargo.stdout(Stdio::piped());
        }

        if let Some(manifest_path) = self.manifest_path.as_ref() {
            cargo.arg("--manifest-path").arg(manifest_path);
        }

        cargo
    }

    pub fn publish(&self, cli_args: &Cli) -> miette::Result<Child> {
        let mut cargo = self.command(cli_args.suppress.includes_cargo());
        cargo.arg("publish");
        if cli_args.dry_run() {
            cargo.arg("--dry-run");
        }
        let git = GitBuilder::new()
            .root_directory(cli_args.root_dir()?)
            .build();
        git.dirty_files()?;

        if cli_args.no_verify() {
            cargo.arg("--no-verify");
        }

        // BUG: Be able to remove --allow-dirty #1
        cargo.args(["--allow-dirty"]);
        tracing::debug!("Running: {:?}", cargo);
        cargo.spawn().into_diagnostic()
    }

    pub fn generate_lockfile(&self, _cli_args: &Cli) -> miette::Result<()> {
        let mut cargo = self.command(true);
        cargo.arg("generate-lockfile");

        tracing::debug!("Running: {:?}", cargo);
        let output = cargo.output().into_diagnostic()?;
        if !output.status.success() {
            Err(
                miette::miette!("{}", String::from_utf8(output.stderr).into_diagnostic()?)
                    .context("While running `cargo generate-lockfile`"),
            )?;
        }
        Ok(())
    }
}
