pub(crate) mod error;
pub(crate) mod toml_file;
pub(crate) mod version_location;

use miette::Result;
use tracing::{info, instrument, warn};

use crate::{Cli, Packages, display_path, error::ManifestNotFoundError};

#[instrument(skip(args), fields(cargo_file))]
pub fn generate_packages(args: &mut Cli) -> Result<Packages> {
    let mut cli_path = args.manifest.manifest_path.as_ref();
    let mut command = args.manifest.metadata();
    if let Some(manifest_path) = cli_path.as_mut() {
        let mut manifest_path = manifest_path.clone();
        if manifest_path.is_dir() {
            manifest_path.push("Cargo.toml");
        }
        command.manifest_path(manifest_path);
    };
    let metadata = match command.exec() {
        Ok(m) => m,
        Err(e) => {
            use cargo_metadata::Error as CmErr;
            let mut msg = match e {
                CmErr::CargoMetadata { stderr: s } => s,
                CmErr::Io(e) => e.to_string(),
                CmErr::Utf8(utf8_error) => utf8_error.to_string(),
                CmErr::ErrUtf8(_) => "Cargo.toml is not in valid Utf-8.".into(),
                CmErr::Json(error) => error.to_string(),
                CmErr::NoJson => CmErr::NoJson.to_string(),
            };
            msg.retain(|s| s != '\n');
            let source_code = cli_path.as_ref().map(|&s| {
                s.clone()
                    .canonicalize()
                    .unwrap_or(s.clone())
                    .display()
                    .to_string()
                    .strip_prefix("\\\\?\\") // Bug fix
                    .unwrap_or_default()
                    .to_string()
            });
            let source_len = source_code.clone().unwrap_or_default().len();

            Err(ManifestNotFoundError {
                help: "Ensure you run the command in a rust project or use '--manifest-path' to set a Cargo.toml file".into(),
                source_code,
                label: Some((0, source_len)),
                label_msg: "Check path ends with Cargo.toml or one exists in the given folder.",
                msg,
            })?;
            unreachable!()
        }
    };
    let packages = Packages::from(&metadata);
    let cargo_file = packages.root_manifest_path();
    tracing::Span::current().record("cargo_file", display_path!(cargo_file).to_string());
    if cargo_file.exists() {
        info!("Root cargo file exists");
    } else {
        warn!("Not Found");
        // TODO: Validate this error.
        Err(ManifestNotFoundError {
            help: "Cargo could not find the Cargo.toml file, try specifying it with '-P'.".into(),
            source_code: None,
            label: None,
            label_msg: "",
            msg: "Cargo.toml file was not found by Cargo".into(),
        })?;
    }

    Ok(packages)
}
