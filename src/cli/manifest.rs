//! Replica of crate: [clap-cargo](https://github.com/crate-ci/clap-cargo)
use std::path;

use clap::ValueHint;

use crate::cli::CARGO_HEADER;

/// Cargo flag for selecting the relevant crate.
#[derive(Default, Clone, Debug, PartialEq, Eq, clap::Args)]
#[command(about = None, long_about = None)]
pub struct Manifest {
    #[arg(long, name = "PATH", help_heading = CARGO_HEADER, value_hint(ValueHint::FilePath))]
    /// Path to Cargo.toml.
    /// All commands run as if they run in the the directory of the Cargo.toml set.
    pub manifest_path: Option<path::PathBuf>,
}

impl Manifest {
    /// Create a `cargo_metadata::MetadataCommand`
    ///
    /// Note: Requires the features `cargo_metadata`.
    pub fn metadata(&self) -> cargo_metadata::MetadataCommand {
        let mut c = cargo_metadata::MetadataCommand::new();
        if let Some(ref manifest_path) = self.manifest_path {
            c.manifest_path(manifest_path);
        }
        c
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn verify_app() {
        #[derive(Debug, clap::Parser)]
        struct Cli {
            #[command(flatten)]
            manifest: Manifest,
        }

        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn metadata_with_path() {
        let manifest = Manifest {
            manifest_path: Some(path::PathBuf::from("tests/fixtures/simple/Cargo.toml")),
        };
        let metadata = manifest.metadata();
        metadata.exec().unwrap();
        // TODO verify we forwarded correctly.
    }

    #[test]
    fn metadata_without_path() {
        let cwd = path::PathBuf::from("tests/fixtures/simple");
        let manifest = Manifest {
            manifest_path: None,
        };
        let mut metadata = manifest.metadata();
        metadata.current_dir(cwd).exec().unwrap();
        // TODO verify we forwarded correctly.
    }
}
