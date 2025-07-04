use std::env::args;

use cargo_uv::{
    Action, Cargo, CargoFile, Cli, FOOTER, Git, Packages, Result, bump_version, generate_packages,
    set_version, setup_tracing,
};
use clap::CommandFactory;
use miette::{Context, IntoDiagnostic};
use rusty_viking::MietteDefaultConfig;

use clap::FromArgMatches as _;
use tracing::{debug, error, info};
fn main() -> Result<()> {
    // removes uv from from input
    let input = args().filter(|a| a != "uv").collect::<Vec<_>>();
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;
    let mut cli = Cli::command();
    cli = cli.mut_arg("new_version", |a| {
        a.conflicts_with("action")
            .required_if_eq("action", Action::Set)
    });
    cli.set_bin_name("cargo uv");

    let args = Cli::from_arg_matches(&cli.get_matches_from(&input)).into_diagnostic()?;

    setup_tracing(&args)?;
    args.try_allow_dirty()?;
    let meta = args.metadata()?;

    let current_packages: Packages = generate_packages(&args)?;
    let _current_root_package = current_packages.get_root_package_owned().unwrap();

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

    let mut join_handles = vec![];
    if args.git_tag() {
        info!("Generating git tag");

        // TODO: Test to see if in different repo as manifest-path.
        Git::add_cargo_files(&args, new_packages.cargo_file_path())?;
        Git::commit(&args, &new_version)?;
        Git::tag(&args, &new_version, None)?;
        if args.git_push() {
            let mut gpjh = Git::push(&args, &new_version).context("git push")?;
            join_handles.append(&mut gpjh);
        }
        if args.dry_run() {
            Git::tag(&args, &new_version, Some(vec!["--delete"]))?;
        }
    }
    if args.cargo_publish() {
        join_handles.push(Cargo::publish(&args).context("Cargo Publish")?);
    }

    while !join_handles.is_empty() {
        let mut drop_jh = vec![];
        for (i, join_handle) in &mut join_handles.iter_mut().enumerate() {
            match join_handle.try_wait() {
                Ok(Some(es)) => {
                    drop_jh.push(i);
                    debug!("Command {} finish with {}", join_handle.id(), es);
                }
                Ok(None) => {}
                Err(e) => {
                    error!("Error occured while running a command: {}", e)
                }
            }
        }

        for i in drop_jh {
            join_handles.remove(i).wait().into_diagnostic()?;
        }
    }
    Ok(())
}
