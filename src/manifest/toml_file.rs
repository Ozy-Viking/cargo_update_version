use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, bail};
use semver::Version;
use toml_edit::DocumentMut;
use tracing::instrument;

use crate::{
    VersionLocation,
    manifest::error::{CargoFileError, CargoFileErrorKind, VersionlocationError},
};

/// Indicator that the cargo file has been read.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ReadToml;

/// Limits what can be done until the file has been read.
#[derive(Debug)]
pub struct UnreadToml;

#[derive(Debug, Clone)]
pub struct CargoFile<State> {
    path: PathBuf,
    contents: Option<DocumentMut>,
    __state: PhantomData<State>,
}

impl<State: std::hash::Hash> std::hash::Hash for CargoFile<State> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.__state.hash(state);
    }
}

impl<State: Eq> Eq for CargoFile<State> {}

impl<State: PartialEq> PartialEq for CargoFile<State> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.__state == other.__state
    }
}

impl<'a, S> CargoFile<S> {
    pub fn path(&'a self) -> &'a Path {
        &self.path
    }
}

impl CargoFile<UnreadToml> {
    #[instrument]
    pub fn new(path: impl Into<PathBuf> + std::fmt::Debug) -> miette::Result<CargoFile<ReadToml>> {
        let ret: CargoFile<UnreadToml> = Self::new_lazy(path.into());
        let ret: CargoFile<ReadToml> = ret.read_file()?;
        tracing::trace!("New Cargo file: {}", ret.path().display());
        Ok(ret)
    }

    pub fn new_lazy(path: impl Into<PathBuf>) -> CargoFile<UnreadToml> {
        Self {
            path: path.into(),
            contents: None,
            __state: PhantomData::<UnreadToml>,
        }
    }

    #[instrument(skip(self), fields(self.path))]
    pub fn read_file(self) -> miette::Result<CargoFile<ReadToml>> {
        let CargoFile { path, .. } = self;
        let contents = match ::std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(e) => {
                tracing::error!("Failed to read to string: {}", e);
                bail!("Tried to read file to string: {}", e)
            }
        };

        let contents = Some(contents.parse::<DocumentMut>().into_diagnostic()?);
        Ok(CargoFile {
            path,
            contents,
            __state: PhantomData::<ReadToml>,
        })
    }
}

impl CargoFile<ReadToml> {
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
            .map_err(|e: VersionlocationError| {
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
            .map_err(|e: VersionlocationError| {
                let path = (&e).path().to_path_buf();
                CargoFileErrorKind::LocationError(e).to_error(path)
            })
    }

    #[track_caller]
    #[instrument(skip(self))]
    pub fn set_version(&mut self, new_version: Version) -> miette::Result<(), CargoFileError> {
        let cargo_path = self.path().to_path_buf();
        let pack_err = VersionLocation::Package.set_version(self, &new_version);
        let ws_err = VersionLocation::WorkspacePackage.set_version(self, &new_version);

        if let Some(cargo_file_err) = pack_err.err() {
            use crate::manifest::error::VersionLocationErrorKind as VerLocErrKind;
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
            use crate::manifest::error::VersionLocationErrorKind as VerLocErrKind;
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
        std::fs::write(&self.path, contents).into_diagnostic()?;
        Ok(())
    }
}
