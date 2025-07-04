use std::{marker::PhantomData, path::Path};

use miette::{IntoDiagnostic, bail};
use semver::Version;
use toml_edit::DocumentMut;
use tracing::instrument;

use crate::current_span;

/// Indicator that the cargo file has been read.
#[derive(Debug)]
pub struct ReadToml;

/// Limits what can be done until the file has been read.
#[derive(Debug)]
pub struct NeedToReadToml;

#[derive(Debug)]
pub struct CargoFile<'a, State> {
    path: &'a Path,
    contents: Option<DocumentMut>,
    __state: PhantomData<State>,
}

impl<'a> CargoFile<'a, NeedToReadToml> {
    #[instrument]
    pub fn new(path: &'a Path) -> miette::Result<CargoFile<'a, ReadToml>> {
        let ret = Self::new_lazy(path);
        ret.read_file()
    }

    pub fn new_lazy(path: &'a Path) -> CargoFile<'a, NeedToReadToml> {
        Self {
            path,
            contents: None,
            __state: PhantomData::<NeedToReadToml>,
        }
    }

    #[instrument(skip(self), fields(self.path))]
    pub fn read_file(self) -> miette::Result<CargoFile<'a, ReadToml>> {
        let contents = match ::std::fs::read_to_string(self.path) {
            Ok(contents) => contents,
            Err(e) => {
                tracing::error!("Failed to read to string: {}", e);
                bail!("Tried to read file to string: {}", e)
            }
        };

        let contents = Some(contents.parse::<DocumentMut>().into_diagnostic()?);
        Ok(CargoFile {
            path: self.path,
            contents,
            __state: PhantomData::<ReadToml>,
        })
    }
}
impl<'a> CargoFile<'a, ReadToml> {
    #[instrument(skip_all, fields(version))]
    pub fn get_root_package_version(&mut self) -> Option<Version> {
        let span = current_span!();
        let document = self.contents.as_ref().unwrap();
        let version_item = document.get("package")?.get("version")?;
        span.record("version", version_item.to_string());
        tracing::info!("Current package version found.");
        Some(Version::parse(version_item.as_str().expect("Should always be a string")).unwrap())
    }

    #[instrument(skip(self))]
    pub fn set_root_package_version(&mut self, new_version: &Version) -> miette::Result<()> {
        let doc = self.contents.as_mut().unwrap();

        #[allow(unused_mut)]
        let mut package_table = doc.get_mut("package").unwrap().as_table_mut().unwrap();
        if let Some(version) = package_table.get_mut("version") {
            let version_val = version.as_value_mut().unwrap();
            *version_val = new_version.to_string().into();
            // TODO: Add a flag to enable comment.
            // version_val
            //     .decor_mut()
            //     .set_suffix(" # Modified by cargo-uv");
        }
        Ok(())
    }

    #[instrument(skip(self))]
    pub fn write_cargo_file(&mut self) -> miette::Result<()> {
        let contents = self.contents.as_ref().unwrap().to_string();
        std::fs::write(self.path, contents).into_diagnostic()?;
        Ok(())
    }
}
