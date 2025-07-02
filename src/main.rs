#![doc = include_str!("../README.md")]

pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod git_tag;
pub(crate) mod manifest;

use miette::{IntoDiagnostic, bail};
use rusty_viking::MietteDefaultConfig;
use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;

use crate::{
    cli::Cli,
    git_tag::Git,
    manifest::{find_matifest_path, set_version},
};

static FOOTER: &'static str = "If the bug continues raise an issue on github.";

fn main() -> miette::Result<()> {
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;

    let args = cli::Cli::parse()?;
    setup_tracing(&args)?;

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
        cargo_file.write_cargo_file()?;
    }
    if args.git_tag() {
        Git::add_cargo_file(&args, packages.cargo_file_path())?;
        Git::commit(&args, &new_version)?;
        Git::tag(&args, &new_version, None)?;
        if args.git_push() {
            Git::push(&args, &new_version)?;
        }
        if args.dry_run() {
            Git::tag(&args, &new_version, Some(vec!["--delete"]))?;
        }
    }
    if args.publish() {}
    Ok(())
}

fn setup_tracing(args: &Cli) -> miette::Result<()> {
    let app_level = match args.verbosity().tracing_level() {
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

#[macro_export]
macro_rules! current_span {
    () => {
        tracing::span::Span::current()
    };
}
