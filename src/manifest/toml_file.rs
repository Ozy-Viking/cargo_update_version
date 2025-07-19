use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    panic::Location,
    path::{Path, PathBuf},
};

use miette::{Context, Diagnostic, IntoDiagnostic, bail};
use semver::Version;
use toml_edit::DocumentMut;
use tracing::instrument;

use crate::{VersionLocation, manifest::version_location::VersionlocationError};

/// Indicator that the cargo file has been read.
#[derive(Debug)]
pub struct ReadToml;

/// Limits what can be done until the file has been read.
#[derive(Debug)]
pub struct UnreadToml;

#[derive(Debug)]
pub struct CargoFile<'a, State> {
    path: &'a Path,
    contents: Option<DocumentMut>,
    __state: PhantomData<State>,
}

impl<'a, S> CargoFile<'a, S> {
    pub fn path(&self) -> &'a Path {
        self.path
    }
}

impl<'a> CargoFile<'a, UnreadToml> {
    #[instrument]
    pub fn new(path: &'a Path) -> miette::Result<CargoFile<'a, ReadToml>> {
        let ret = Self::new_lazy(path);
        ret.read_file()
    }

    pub fn new_lazy(path: &'a Path) -> CargoFile<'a, UnreadToml> {
        Self {
            path,
            contents: None,
            __state: PhantomData::<UnreadToml>,
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
    pub fn contents(&self) -> Option<&DocumentMut> {
        self.contents.as_ref()
    }
    pub fn contents_mut(&mut self) -> Option<&mut DocumentMut> {
        self.contents.as_mut()
    }
    #[instrument(skip_all)]
    pub fn get_package_version(&self) -> Option<Version> {
        VersionLocation::Package.get_version(&self).ok()
    }
    #[instrument(skip_all)]
    pub fn get_workspace_version(&self) -> Option<Version> {
        VersionLocation::WorkspacePackage.get_version(&self).ok()
    }

    #[track_caller]
    #[instrument(skip(self))]
    pub fn set_package_version(
        &mut self,
        new_version: &Version,
    ) -> miette::Result<(), CargoFileError> {
        VersionLocation::Package
            .set_version(self, new_version)
            .map_err(|e| {
                let path = (&e).path().to_path_buf();
                CargoFileErrorKind::LocationError(e).to_error(path)
            })
    }

    #[track_caller]
    #[instrument(skip(self))]
    pub fn set_workspace_version(
        &mut self,
        new_version: &Version,
    ) -> miette::Result<(), CargoFileError> {
        VersionLocation::WorkspacePackage
            .set_version(self, new_version)
            .map_err(|e| {
                let path = (&e).path().to_path_buf();
                CargoFileErrorKind::LocationError(e).to_error(path)
            })
    }

    #[track_caller]
    #[instrument(skip(self))]
    pub fn set_version<'s>(
        &'s mut self,
        new_version: &Version,
    ) -> miette::Result<(), CargoFileError<'s>> {
        let cargo_path = self.path().to_path_buf();
        let pack_err = VersionLocation::Package.set_version(self, new_version);
        let ws_err = VersionLocation::WorkspacePackage.set_version(self, new_version);

        if let Some(cargo_file_err) = pack_err.err() {
            use crate::manifest::version_location::VersionLocationErrorKind as VerLocErrKind;
            match &cargo_file_err.kind() {
                VerLocErrKind::SetByWorkspace => (),
                VerLocErrKind::PackageNotFound => (),
                VerLocErrKind::NotFound(_) => (),
                VerLocErrKind::WorkspaceNotFound => unreachable!("Setting package"),
                VerLocErrKind::ItemInvalid(_) | VerLocErrKind::SemverError(_) => {
                    return Err(
                        CargoFileErrorKind::LocationError(cargo_file_err).to_error(cargo_path)
                    );
                }
            }
        } else {
            return Ok(());
        };

        if let Some(ver_loc_error) = ws_err.err() {
            use crate::manifest::version_location::VersionLocationErrorKind as VerLocErrKind;
            match ver_loc_error.kind() {
                VerLocErrKind::SetByWorkspace => unreachable!(),
                VerLocErrKind::NotFound(_) => {
                    Err(CargoFileErrorKind::NoPackageOrWorkspaceVersion.to_error(cargo_path))
                }
                VerLocErrKind::PackageNotFound => unreachable!(),
                VerLocErrKind::WorkspaceNotFound => {
                    Err(CargoFileErrorKind::NoPackageOrWorkspaceVersion.to_error(cargo_path))
                }
                VerLocErrKind::ItemInvalid(_) | VerLocErrKind::SemverError(_) => {
                    Err(CargoFileErrorKind::LocationError(ver_loc_error).to_error(cargo_path))
                }
            }
        } else {
            Ok(())
        }
    }

    #[instrument(skip(self))]
    pub fn write_cargo_file(&mut self) -> miette::Result<()> {
        let contents = self.contents.as_ref().unwrap().to_string();
        std::fs::write(self.path, contents).into_diagnostic()?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, Diagnostic)]
pub struct CargoFileError<'e> {
    #[source]
    kind: CargoFileErrorKind,
    path: PathBuf,
    #[help("Call location: {location_called}")]
    location_called: &'e Location<'static>,
}

impl<'e> CargoFileError<'e> {
    pub fn kind(&self) -> &CargoFileErrorKind {
        &self.kind
    }
}

impl Display for CargoFileError<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Cargo.toml: {}", self.path.display())
    }
}

// #[allow(dead_code)]
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CargoFileErrorKind {
    #[error("No root package")]
    NoRootPackage,
    #[error("No workspace package version")]
    NoWorkspaceVersion,
    #[error("No package version")]
    NoPackageVersion,
    #[error("No workspace or root package version is present")]
    NoPackageOrWorkspaceVersion,
    #[error("This package version is set by the workspace")]
    SetByWorkspace,
    #[error("Cargo file error with Version Location")]
    LocationError(#[from] VersionlocationError),
}

impl CargoFileErrorKind {
    #[track_caller]
    pub fn to_error<'e>(self, path: impl Into<PathBuf>) -> CargoFileError<'e> {
        CargoFileError {
            kind: self,
            path: path.into(),
            location_called: Location::caller(),
        }
    }
}
