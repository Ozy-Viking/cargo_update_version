#![doc = include_str!("../README.md")]

pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod git_tag;
pub(crate) mod manifest;

use rusty_viking::MietteDefaultConfig;
use tracing::Level;

use crate::{
    git_tag::Git,
    manifest::{find_matifest_path, set_version},
};

static FOOTER: &'static str = "If the bug continues raise an issue on github.";

fn main() -> miette::Result<()> {
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;

    let args = cli::Cli::parse()?;

    rusty_viking::tracing::setup_tracing(
        module_path!(),
        args.verbosity()
            .tracing_level()
            .expect("there should be a level"),
        Level::WARN,
        None,
    )?;
    // Git::is_dirty()?;
    let cargo_manifest = manifest::find_matifest_path(args.manifest_path())?;
    let old_version = cargo_manifest
        .get_root_package()
        .expect("Currently under this belief")
        .version();

    use cli::BumpVersion as BV;
    let packages = find_matifest_path(args.manifest_path())?;
    let mut cargo_file = manifest::CargoFile::new(packages.cargo_file_path())?;
    assert_eq!(&cargo_file.get_root_package_version().unwrap(), old_version);
    let new_packages = match args.bump_version() {
        BV::Patch | BV::Minor | BV::Major => {
            manifest::bump_version(args.bump_version(), cargo_manifest)?
        }
        BV::Set(version) => set_version(cargo_manifest, version)?,
    };

    cargo_file.set_root_package_version(new_packages.get_root_package().unwrap().version())?;
    cargo_file.write_cargo_file()?;
    Git::add_cargo_file(packages.cargo_file_path())?;
    Git::commit(args.git_message())?;

    Ok(())
}

#[macro_export]
macro_rules! current_span {
    () => {
        tracing::span::Span::current()
    };
}
