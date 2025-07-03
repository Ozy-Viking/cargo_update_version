#![doc = include_str!("../README.md")]

pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod git_tag;
pub(crate) mod manifest;

use std::process::{Child, Command};

use clap::Parser;
use miette::{Context, IntoDiagnostic, bail};
use rusty_viking::MietteDefaultConfig;
use tracing::{Level, debug, error, info};
use tracing_subscriber::util::SubscriberInitExt;

use crate::{
    cli::Cli,
    git_tag::Git,
    manifest::{find_matifest_path, set_version},
};

static FOOTER: &str = "If the bug continues, raise an issue on github: https://github.com/Ozy-Viking/cargo_update_version/issues";

fn main() -> miette::Result<()> {
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;

    let args = cli::cli::Cli::parse();
    setup_tracing(&args)?;
    let meta = args.metadata()?;
    match args.action() {
        cli::cli::Action::Patch => {}
        cli::cli::Action::Minor => todo!(),
        cli::cli::Action::Major => todo!(),
        cli::cli::Action::Set => todo!(),
        cli::cli::Action::Print => {
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
    }

    return Ok(());

    /*
        let cargo_manifest = manifest::find_matifest_path(args.manifest_path())?;
        if args.print_version() {
            let version = cargo_manifest
                .get_root_package()
                .expect("Currently under this belief")
                .version();
            print!("{}", version);
            return Ok(());
        }

        if !args.allow_dirty() {
            Git::is_dirty()?;
        }
        let old_version = cargo_manifest
            .get_root_package()
            .expect("Currently under this belief")
            .version();

        use cli::BumpVersion as BV;
        let packages = find_matifest_path(args.manifest_path())?;
        let mut cargo_file = manifest::CargoFile::new(packages.cargo_file_path())?;
        assert_eq!(&cargo_file.get_root_package_version().unwrap(), old_version);
        let new_packages = match args.bump_version() {
            BV::Patch | BV::Minor | BV::Major => manifest::bump_version(&args, cargo_manifest)?,
            BV::Set(version) => set_version(cargo_manifest, version)?,
        };
        let new_version = new_packages
            .get_root_package_version()
            .expect("Assuming only root version ops.");

        cargo_file.set_root_package_version(&new_version)?;
        if !args.dry_run() {
            info!("Writing Cargo File");
            cargo_file.write_cargo_file()?;
        }
        let mut join_handles = vec![];
        if args.git_tag() {
            info!("Generating git tag");
            Git::add_cargo_files(&args, packages.cargo_file_path())?;
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
        if args.publish() {
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
    */
}

fn setup_tracing(args: &cli::cli::Cli) -> miette::Result<()> {
    let app_level = match args.tracing_level() {
        Some(l) => l,
        None => bail!(
            help = "Raise issue in github please or try a different verbosity level.",
            "Tracing level not set somehow."
        ),
    };

    let target = tracing_subscriber::filter::Targets::new()
        .with_target(module_path!(), app_level)
        .with_default(Level::ERROR);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(target.to_string()));

    #[allow(unused_mut)]
    let mut builder = tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(env_filter);
    #[cfg(debug_assertions)]
    {
        builder = builder.with_line_number(true).with_file(true)
    }
    builder.finish().try_init().into_diagnostic()?;
    Ok(())
}

struct Cargo;
impl Cargo {
    fn command() -> Command {
        Command::new("cargo")
    }

    fn publish(cli_args: &Cli) -> miette::Result<Child> {
        let mut cargo = Cargo::command();
        cargo.arg("publish");
        if cli_args.dry_run() {
            cargo.arg("--dry-run");
        }
        Git::is_dirty()?;
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

#[macro_export]
macro_rules! current_span {
    () => {
        tracing::span::Span::current()
    };
}
