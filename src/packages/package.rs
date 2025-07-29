use crate::{
    Action, Bumpable, CargoFile, PackageName, ReadToml, Result, VersionLocation, current_span,
    manifest::version_location::VersionType,
};
use miette::bail;
use semver::{BuildMetadata, Prerelease, Version};
use std::path::{Path, PathBuf};
use tracing::instrument;

#[derive(Debug, Eq, Clone)]
pub struct Package<CargoFileState> {
    name: PackageName,
    version_type: VersionType,
    version: Version,
    manifest_path: PathBuf,
    cargo_file: CargoFile<CargoFileState>,
}

impl<CargoFileState: PartialEq> PartialEq for Package<CargoFileState> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.manifest_path == other.manifest_path
    }
}

impl<CargoFileState: std::hash::Hash> std::hash::Hash for Package<CargoFileState> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        // self.version_type.hash(state);
        // self.manifest_path.hash(state);
        // self.cargo_file.hash(state);
    }
}

impl<CargoFileState> Package<CargoFileState> {
    pub fn name(&self) -> &PackageName {
        &self.name
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.version
    }

    pub fn manifest_path(&self) -> &PathBuf {
        &self.manifest_path
    }

    pub fn cargo_file(&self) -> &CargoFile<CargoFileState> {
        &self.cargo_file
    }

    pub fn cargo_file_mut(&mut self) -> &mut CargoFile<CargoFileState> {
        &mut self.cargo_file
    }

    pub fn version_type(&self) -> VersionType {
        self.version_type
    }
}

impl From<cargo_metadata::Package> for Package<ReadToml> {
    fn from(meta_package: cargo_metadata::Package) -> Package<ReadToml> {
        let manifest_path: PathBuf = meta_package.manifest_path.into();
        let cargo_file = CargoFile::new(manifest_path.clone()).expect("from cargo manifest");
        Self {
            name: meta_package.name.to_string().into(),
            version: meta_package.version,
            version_type: Package::set_version_type(&cargo_file)
                .expect("Cargo manifest run with no error"),
            cargo_file: cargo_file,
            manifest_path,
        }
    }
}
impl Package<ReadToml> {
    #[instrument(skip_all)]
    pub fn set_version_type(cargo_file: &CargoFile<ReadToml>) -> Result<VersionType> {
        let package = match VersionLocation::Package.get_version(cargo_file) {
            Ok(_v) => Ok(VersionType::Package),
            Err(e) => match e.kind() {
                crate::VersionLocationErrorKind::SetByWorkspace => Ok(VersionType::SetByWorkspace),
                _ => Err(e),
            },
        };

        if let Ok(ver_type) = package {
            return Ok(ver_type);
        }

        let ws = match VersionLocation::WorkspacePackage.get_version(cargo_file) {
            Ok(_v) => VersionType::WorkspacePackage,
            Err(e) => {
                tracing::error!("Invalid version: {}", cargo_file.path().display());
                return Err(miette::miette!(e)
                    .wrap_err(package.map_err(|e| e.to_string()).err().unwrap_or_default()));
            }
        };
        Ok(ws)
    }

    pub fn set_version(&mut self, version: Version) -> Result<Version> {
        self.version = version.clone();
        let cargo_file = self.cargo_file_mut();
        let res = cargo_file.set_version(version);
        res?;
        Ok(self.version().clone())
    }

    #[instrument(skip(self, pre, build), fields(from, to))]
    pub fn bump_version(
        &mut self,
        action: Action,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force: bool,
    ) -> Result<Version> {
        let span = current_span!();
        span.record("from", self.version.to_string());
        let name = self.name().clone();
        tracing::trace!("Package {}: Bump Version", name);

        let version = self.version_mut();
        let new_version = version.bump(action, pre, build, force)?;
        self.cargo_file_mut().set_version(new_version)?;
        span.record("to", self.version().to_string());
        println!("{name}: {}", self.version());
        Ok(self.version().clone())
    }

    pub fn write_cargo_file(&mut self) -> Result<Version> {
        if self.version_type() == VersionType::SetByWorkspace {
            let msg = format!(
                "Can't modify SetByWorkspace version from a bool: {}",
                self.manifest_path.as_os_str().display()
            );
            tracing::error!("{}", msg);
            bail!("{msg}")
        }
        self.cargo_file_mut().write_cargo_file()?;
        match self.version_type() {
            VersionType::Package => Ok(VersionLocation::Package.get_version(self.cargo_file())?),
            VersionType::SetByWorkspace => unreachable!(),

            VersionType::WorkspacePackage => {
                Ok(VersionLocation::WorkspacePackage.get_version(self.cargo_file())?)
            }
        }
    }

    #[track_caller]
    pub fn workspace_package(manifest_path: &Path) -> Result<Package<ReadToml>> {
        let cargo_file = CargoFile::new(manifest_path)?;
        let version = VersionLocation::WorkspacePackage.get_version(&cargo_file)?;

        Ok(Package {
            name: PackageName("workspace.package".into()),
            version_type: VersionType::WorkspacePackage,
            version,
            manifest_path: manifest_path.into(),
            cargo_file,
        })
    }
}
