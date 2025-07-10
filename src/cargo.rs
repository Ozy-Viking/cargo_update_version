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
        cargo.stderr(Stdio::piped());
        cargo
    }

    pub fn publish(cli_args: &Cli) -> miette::Result<Child> {
        let mut cargo = Cargo::command(cli_args.supress_stdout);
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
        tracing::trace!("About to run: {:?}", &cargo);
        cargo.spawn().into_diagnostic()
    }
}
