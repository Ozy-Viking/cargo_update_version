use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use miette::IntoDiagnostic;
use tracing::{debug, instrument};

use crate::{Process, cli::Suppress};

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

    pub fn publish(
        &self,
        suppress: Suppress,
        dry_run: bool,
        no_verify: bool,
        allow_dirty: bool,
    ) -> miette::Result<Child> {
        let mut cargo = self.command(suppress.includes_cargo());
        cargo.arg("publish");
        if dry_run {
            cargo.arg("--dry-run");
        }

        if no_verify {
            cargo.arg("--no-verify");
        }

        // BUG: Be able to remove --allow-dirty #1
        // cargo.args(["--allow-dirty"]);
        if allow_dirty {
            cargo.args(["--allow-dirty"]);
        }
        Process::Spawn.run(cargo)?.try_into_child()
    }

    pub fn generate_lockfile(&self) -> miette::Result<()> {
        let mut cargo = self.command(true);
        cargo.arg("generate-lockfile");

        let output = Process::Output.run(cargo)?.try_into_output()?;
        if !output.status.success() {
            Err(
                miette::miette!("{}", String::from_utf8(output.stderr).into_diagnostic()?)
                    .context("While running `cargo generate-lockfile`"),
            )?;
        }
        Ok(())
    }
}
