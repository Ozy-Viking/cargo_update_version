#![doc = include_str!("../README.md")]

pub(crate) mod cargo;
pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod git;
pub(crate) mod manifest;
pub(crate) mod packages;
pub(crate) mod process;
pub(crate) mod task;
pub(crate) mod version;

pub use cargo::Cargo;
pub use cli::{Action, Branch, Cli};
pub use git::{Git, GitBuilder, GitFile, GitFiles, NoRootDirSet, Stash};
pub use manifest::error::{
    CargoFileError, CargoFileErrorKind, ItemType, VersionLocationErrorKind, VersionlocationError,
};
pub use manifest::generate_packages;
pub use manifest::toml_file::{CargoFile, ReadToml, UnreadToml};
pub use manifest::version_location::{VersionLocation, VersionType};
pub use miette::Result;
pub use packages::{Package, PackageError, PackageName, Packages};
pub use process::{OutputExt, Process, ProcessOutput};
pub use task::{Task, TaskError, Tasks};
pub use version::{Bumpable, Setable};

use miette::{IntoDiagnostic, bail};
use tracing::Level;
use tracing_subscriber::util::SubscriberInitExt;

/// Footer for the [miette::MietteHandler]
pub static FOOTER: &str = "If the bug continues, raise an issue on github: https://github.com/Ozy-Viking/cargo_update_version/issues";

pub fn setup_tracing(args: &Cli) -> miette::Result<()> {
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

#[macro_export]
macro_rules! current_span {
    () => {
        tracing::span::Span::current()
    };
}

#[macro_export]
macro_rules! display_path {
    ($path:ident) => {
        $path.as_os_str().display()
    };
}
