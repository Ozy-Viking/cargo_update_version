use std::env::args;

use cargo_uv::{Cli, FOOTER, Packages, Result, Tasks, exit, setup_tracing};
use rusty_viking::MietteDefaultConfig;

fn main() -> Result<()> {
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;
    let args = args().collect();
    let mut cli_args = Cli::cli_args(args, Some("cargo uv"), Some("uv"))?;
    setup_tracing(&cli_args)?;

    let packages = Packages::from(cli_args.get_metadata()?);
    let mut tasks = Tasks::generate_tasks(&mut cli_args, packages)?;

    tasks = tasks.run_all(&cli_args)?.join_all()?;
    tracing::info!("Completed run, starting cleanup");
    tasks.run_cleanup_tasks(&cli_args)?;

    exit!();
}
