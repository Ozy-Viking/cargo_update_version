use std::process::{Child, Command, Stdio};

use miette::IntoDiagnostic;
use tracing::{debug, instrument};

use crate::{GitBuilder, cli::Cli};

pub struct Cargo;
impl Cargo {
    #[instrument(name = "Cargo::command")]
    pub fn command(supress_stdout: bool) -> Command {
        let mut cargo = Command::new("cargo");
        if !supress_stdout {
            debug!("Inherit");
            cargo.stdout(Stdio::inherit());
        } else {
            cargo.stdout(Stdio::piped());
        }
        cargo
    }

    pub fn publish(cli_args: &Cli) -> miette::Result<Child> {
        let mut cargo = Cargo::command(cli_args.suppress.includes_cargo());
        cargo.arg("publish");
        if cli_args.dry_run() {
            cargo.arg("--dry-run");
        }
        let git = GitBuilder::new()
            .root_directory(cli_args.root_dir()?)
            .build();
        git.dirty_files()?;
        if cli_args.manifest.manifest_path.is_some() {
            cargo
                .arg("--manifest-path")
                .arg(cli_args.manifest.manifest_path.clone().unwrap());
        }
        if cli_args.no_verify() {
            cargo.arg("--no-verify");
        }

        // BUG: Be able to remove --allow-dirty #1
        cargo.args(["--allow-dirty"]);
        tracing::debug!("Running: {:?}", cargo);
        cargo.spawn().into_diagnostic()
    }

    pub fn generate_lockfile(cli_args: &Cli) -> miette::Result<()> {
        let mut cargo = Cargo::command(true);
        cargo.arg("generate-lockfile");
        if cli_args.manifest.manifest_path.is_some() {
            cargo
                .arg("--manifest-path")
                .arg(cli_args.manifest.manifest_path.clone().unwrap());
        }

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
