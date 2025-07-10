use std::env::args;

use cargo_uv::{
    Action, Cargo, CargoFile, Cli, FOOTER, GitBuilder, Packages, Result, Tasks, bump_version,
    generate_packages, set_version, setup_tracing,
};
use clap::CommandFactory;
use miette::{Context, IntoDiagnostic};
use rusty_viking::MietteDefaultConfig;

use clap::FromArgMatches as _;
use tracing::info;
fn main() -> Result<()> {
    // removes uv from from input
    let input = args().filter(|a| a != "uv").collect::<Vec<_>>();
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;
    let mut cli = Cli::command();
    cli = cli.mut_arg("set_version", |a| a.required_if_eq("action", Action::Set));
    cli.set_bin_name("cargo uv");
    cli = cli.next_line_help(false);

    let args = Cli::from_arg_matches(&cli.get_matches_from(&input)).into_diagnostic()?;

    setup_tracing(&args)?;
    args.try_allow_dirty()?;
    let meta = args.metadata()?;

    let current_packages: Packages = generate_packages(&args)?;

    let new_packages = match args.action() {
        Action::Set => set_version(current_packages, &args)?,
        Action::Print => {
            let version = meta
                .root_package()
                .ok_or(miette::miette!(
                    "No root package. Currently only projects with a root package is supported."
                ))?
                .version
                .clone();

            println!("{}", version);
            return Ok(());
        }
        Action::Major | Action::Minor | Action::Patch => bump_version(&args, current_packages)?,
    };

    let new_version = new_packages
        .get_root_package()
        .expect("Only dealing with root packages.")
        .version()
        .clone();

    let mut cargo_file = CargoFile::new(new_packages.cargo_file_path())?;

    cargo_file.set_root_package_version(&new_version)?;

    if !args.dry_run() {
        info!("Writing Cargo File");
        cargo_file.write_cargo_file()?;
    }

    let _new_manifest = args.metadata()?;

    let mut tasks = Tasks::new();
    if args.git_tag() {
        info!("Generating git tag");
        let root_dir = args.root_dir()?;
        let git = GitBuilder::new().root_directory(root_dir).build();

        git.add_cargo_files(new_packages.cargo_file_path())?;
        todo!();
        git.commit(&args, &new_version)?;
        git.tag(&args, &new_version, None)?;
        if args.git_push() {
            let gpjh = git.push(&args, &new_version).context("git push")?;
            tasks.append(gpjh);
        }
        if args.dry_run() {
            git.tag(&args, &new_version, Some(vec!["--delete"]))?;
        }
    }
    if args.cargo_publish() {
        tasks.insert(
            cargo_uv::Task::Publish,
            Some(Cargo::publish(&args).context("Cargo Publish")?),
        );
    }

    match tasks.join_all() {
        Ok(_ts) => {
            println!("{}", new_version);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Tasks errored, Completed tasks: {:?}", e.completed_tasks);
            tracing::warn!("Tasks with unknown status: {:?}", e.incomplete_tasks);
            Err(e.into())
        }
    }
}
