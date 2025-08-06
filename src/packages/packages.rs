use std::io::Write;
use std::path::Path;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use cargo_metadata::Metadata;
use indexmap::IndexSet;
use miette::Context;
use semver::Version;
use tracing::{debug, instrument};

use super::{Package, PackageError, PackageName};
use crate::{ReadToml, Result, VersionLocation, display_path};

#[derive(Debug, Clone, PartialEq)]
pub struct Packages {
    root_directory: PathBuf,
    root_cargo_toml: PathBuf,
    root_cargo_lock: PathBuf,

    /// Root package of the rust project.
    /// BUG: #49 When both root_package and workspace_package they overwrite the root_manifest
    root_package: Option<PackageName>,

    /// The root package version or if not present the workplace version.
    root_version: Option<Version>,

    /// Default members
    default_members: HashSet<PackageName>,

    /// Hashmap of the packages in the rust project.
    packages: HashMap<PackageName, Package<ReadToml>>,

    /// The workspace.package of the root Cargo.toml
    workspace_package: Option<Package<ReadToml>>,
}

impl AsRef<HashMap<PackageName, Package<ReadToml>>> for Packages {
    fn as_ref(&self) -> &HashMap<PackageName, Package<ReadToml>> {
        &self.packages
    }
}

impl Packages {
    pub fn new<P, N>(
        root_directory: PathBuf,
        packages: &[P],
        root_package: Option<&P>,
        default_members: Vec<N>,
    ) -> Result<Self>
    where
        P: Into<Package<ReadToml>> + Clone,
        N: ToString + Clone,
    {
        let root_cargo_toml = root_directory.join("Cargo.toml");
        let root_cargo_lock = root_directory.join("Cargo.lock");
        let workspace_package = Package::workspace_package(&root_cargo_toml).ok();

        let mut ret = Self {
            root_directory: root_directory.clone(),
            root_package: None,
            root_version: None,
            packages: HashMap::from_iter(packages.iter().map(|package| {
                let package: Package<ReadToml> = package.clone().into();
                (package.name().clone(), package)
            })),
            default_members: HashSet::from_iter(
                default_members.iter().map(|n| PackageName(n.to_string())),
            ),
            workspace_package,
            root_cargo_toml,
            root_cargo_lock,
        };
        if let Some(root_package) = root_package {
            let root_package: Package<ReadToml> = root_package.clone().into();
            ret.root_package = ret.set_root_package_name(root_package.name()).cloned();
            ret.root_version = Some(root_package.version().clone());
        }
        Ok(ret)
    }

    #[instrument(skip_all)]
    #[deprecated(since = "0.7.0", note = "Changed to root_manifest_path")]
    pub fn cargo_file_path(&self) -> &Path {
        debug!("Fetching cargo path");
        self.root_manifest_path()
    }

    pub fn drop_root_package_name(&mut self) {
        self.root_package = None
    }

    pub fn workspace_default_members(&self) -> HashSet<&PackageName> {
        self.packages()
            .keys()
            .filter(|&p| self.default_members.contains(p))
            .collect()
    }

    pub fn workspace_members(&self) -> HashSet<&PackageName> {
        self.packages.keys().collect()
    }

    /// Trys to set root package name, returns [None] if package doesn't exist or an invalid name is inputed.
    ///
    /// Use [Packages::try_set_root_package_name] if you want to see the error.
    pub fn set_root_package_name<'name>(
        &mut self,
        package_name: &'name PackageName,
    ) -> Option<&'name PackageName> {
        self.try_set_root_package_name(package_name).ok()
    }

    pub fn try_set_root_package_name<'name>(
        &mut self,
        package_name: &'name PackageName,
    ) -> miette::Result<&'name PackageName> {
        if package_name.is_empty() {
            Err(PackageError::PackageNameNotProvided)?
        }
        match self.packages.contains_key(package_name) {
            true => {
                self.root_package = Some(package_name.clone());
                Ok(package_name)
            }
            false => Err(PackageError::PackageNameNotFound(package_name.clone()))?,
        }
    }

    /// Only needs a `&self` as it returns what is set.
    pub fn root_package_name_unchecked(&self) -> Option<&PackageName> {
        self.root_package.as_ref()
    }

    /// Validated the set root name, uses [Packages::drop_root_package_name] if root_name is invalid.
    pub fn root_package_name(&mut self) -> Option<&PackageName> {
        let root_name = self.root_package.clone()?;
        if self.packages.contains_key(&root_name) {
            self.root_package.as_ref()
        } else {
            self.drop_root_package_name();
            None
        }
    }

    /// Checks whether there is a root name and returns a ref to the [root_package][Package].
    pub fn get_root_package(&self) -> Option<&Package<ReadToml>> {
        let root_name = self.root_package_name_unchecked()?;
        self.packages.get(root_name)
    }

    /// Checks whether there is a root name and returns a mut ref to the [root_package][Package].
    pub fn get_root_package_mut(&mut self) -> Option<&mut Package<ReadToml>> {
        let root_name = self.root_package_name_unchecked()?.clone();
        self.packages.get_mut(&root_name)
    }

    /// Checks whether there is a root name and returns an owned clone of the [root_package][Package].
    pub fn get_root_package_owned(&self) -> Option<Package<ReadToml>> {
        let root_name = self.root_package_name_unchecked()?;
        self.packages.get(root_name).cloned()
    }

    /// Uses the [HashMap::get] and returns a ref to the [Package] if it exists.
    pub fn get_package(&self, package_name: &PackageName) -> Option<&Package<ReadToml>> {
        if package_name.is_workspace_package() {
            self.get_root_package()
        } else {
            self.packages.get(package_name)
        }
    }

    ///  Uses the [HashMap::get_mut] and returns a mut ref to the [Package] if it exists.
    pub fn get_package_mut(
        &mut self,
        package_name: &PackageName,
    ) -> Option<&mut Package<ReadToml>> {
        if package_name.is_workspace_package() {
            self.workspace_package_mut()
        } else {
            self.packages.get_mut(package_name)
        }
    }

    ///  Uses the [HashMap::get] followed by [Option::cloned] and returns [Package] if it exists.
    pub fn get_package_owned(&self, package_name: &PackageName) -> Option<Package<ReadToml>> {
        self.packages.get(package_name).cloned()
    }

    pub fn packages(&self) -> &HashMap<PackageName, Package<ReadToml>> {
        &self.packages
    }

    pub fn package_set(&self) -> HashSet<&Package<ReadToml>> {
        self.packages.values().collect::<HashSet<_>>()
    }

    pub fn package_set_mut(&mut self) -> HashSet<&mut Package<ReadToml>> {
        self.packages.values_mut().collect::<HashSet<_>>()
    }

    pub fn get_root_package_version(&self) -> Option<Version> {
        self.get_root_package().map(|rp| rp.version().clone())
    }

    /// Determine what the root version is for the packages.
    ///
    /// Order of checks:
    /// - root version is set
    /// - root package version
    /// - workplace.package.version
    /// - TODO: If all versions are the same use that version
    ///
    pub fn root_version(&self) -> Result<Version, PackageError> {
        let error_no_root_package = PackageError::NoRootVersion;
        if self.root_version.is_some() {
            return Ok(self
                .root_version
                .as_ref()
                .expect("There is a root version")
                .clone());
        };

        // Checking the root package
        if let Some(root_package_name) = &self.root_package {
            let root_package = self.packages.get(root_package_name);
            if let Some(root_package) = root_package {
                return Ok(root_package.version().clone());
            };
        };

        if let Some(workspace_package) = self.workspace_package.as_ref() {
            // Checking the workspace package
            match VersionLocation::WorkspacePackage.get_version(workspace_package.cargo_file()) {
                Ok(v) => return Ok(v),
                Err(e) => match e.kind() {
                    crate::VersionLocationErrorKind::SetByWorkspace => unreachable!(),
                    crate::VersionLocationErrorKind::NotFound(_) => (),
                    crate::VersionLocationErrorKind::PackageNotFound => unreachable!(),
                    crate::VersionLocationErrorKind::WorkspaceNotFound => (),
                    crate::VersionLocationErrorKind::ItemInvalid(_) => (),
                    crate::VersionLocationErrorKind::SemverError(_) => (),
                },
            };
        }
        let mut versions = IndexSet::new();
        for ver in self.packages().values().map(|p| p.version()) {
            versions.insert(ver);
        }

        if versions.len() == 1 {
            Ok(versions.pop().unwrap().clone())
        } else {
            Err(error_no_root_package)
        }
    }
    pub fn workspace_package(&self) -> Option<&Package<ReadToml>> {
        self.workspace_package.as_ref()
    }
    pub fn workspace_package_mut(&mut self) -> Option<&mut Package<ReadToml>> {
        self.workspace_package.as_mut()
    }

    pub fn root_cargo_lock_path(&self) -> &Path {
        &self.root_cargo_lock
    }

    pub fn root_manifest_path(&self) -> &Path {
        &self.root_cargo_toml
    }

    pub fn root_directory(&self) -> &Path {
        &self.root_directory
    }
}

impl Packages {
    pub fn display_tree(&self) -> String {
        let mut ret_string = Vec::new();
        let root_package = self.root_package.as_ref();
        let path_base = self.root_directory();
        let make_relative = |package: &Package<ReadToml>| {
            PathBuf::new()
                .join(".")
                .join(
                    package
                        .manifest_path()
                        .parent()
                        .unwrap()
                        .strip_prefix(path_base)
                        .unwrap(),
                )
                .as_os_str()
                .to_string_lossy()
                .into_owned()
        };
        let _ = writeln!(
            ret_string,
            "Workspace root: {}",
            path_base.as_os_str().to_str().unwrap()
        );
        if let Some(root) = root_package {
            let package = self.get_package(root).unwrap();
            let _ = writeln!(ret_string, "Root package: {root} {}", package.version(),);
        }

        if !self.default_members.is_empty() {
            let _ = writeln!(
                ret_string,
                "Default members: {:?}",
                self.default_members
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
            );
        }

        let _ = writeln!(ret_string);

        if let Some(root) = root_package {
            let _ = writeln!(ret_string, "{root}");
        } else {
            let _ = writeln!(ret_string, "Members:");
        }

        let mut items = self.packages.iter().collect::<Vec<_>>();
        items.sort_by_key(|(n, _)| n.0.as_str());
        let last = items.last().cloned();

        for (name, package) in items {
            if Some(name) == root_package {
                continue;
            }
            if Some((name, package)) == last {
                let _ = writeln!(
                    ret_string,
                    "└─ {name} {}: {}",
                    package.version(),
                    make_relative(package)
                );
            } else {
                let _ = writeln!(
                    ret_string,
                    "├─ {name} {}: {}",
                    package.version(),
                    make_relative(package)
                );
            }
        }
        String::from_utf8(ret_string).expect("Chars is valid utf-8")
    }
}

impl From<&Metadata> for Packages {
    #[track_caller]
    #[instrument(skip_all)]
    fn from(metadata: &Metadata) -> Self {
        let root_path = metadata.workspace_root.join("Cargo.toml");
        tracing::debug!(
            "Constructing Packages from Metadata: {}",
            display_path!(root_path)
        );

        let default_members = metadata
            .workspace_default_packages()
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();
        tracing::trace!("Default members {:?}", default_members);
        Self::new(
            metadata.workspace_root.clone().into_std_path_buf(),
            &metadata.packages,
            metadata.root_package(),
            default_members,
        )
        .expect("From cargo metadata")
    }
}

/// Methods used by [`Task::run`]
///
/// [`Task::run`]: crate::Task::run
impl Packages {
    /// Used by both [`Task::Bump`] and [`Task::Set`].
    ///
    /// [`Task::Bump`]: crate::Task::Bump
    /// [`Task::Set`]: crate::Task::Set
    #[instrument(skip(self))]
    pub fn set_package_version(
        &mut self,
        package_name: &PackageName,
        new_version: Version,
    ) -> Result<Version> {
        tracing::trace!("Setting package version.");
        let package = self
            .get_package_mut(package_name)
            .ok_or(miette::miette!("No package by name: {package_name}"))?;
        package.set_version(new_version)
    }

    /// Used by both [`Task::BumpWorkspace`] and [`Task::SetWorkspace`].
    ///
    /// [`Task::BumpWorkspace`]: crate::Task::BumpWorkspace
    /// [`Task::SetWorkspace`]: crate::Task::SetWorkspace
    #[instrument(skip(self))]
    pub fn set_workspace_package_version(&mut self, new_version: Version) -> Result<Version> {
        tracing::trace!("Setting workspace package version.");
        let package = self.workspace_package_mut().ok_or(miette::miette!(
            "Expected 'workspace.package.version' to exist."
        ))?;
        package
            .set_version(new_version)
            .context("setting workspace.package version")
    }

    /// Used by [`Task::WriteCargoToml`]
    ///
    /// [`Task::WriteCargoToml`]: crate::Task::WriteCargoToml
    #[instrument(skip(self))]
    pub fn write_cargo_file(&mut self, package_name: &PackageName) -> Result<()> {
        let package = self
            .get_package_mut(package_name)
            .ok_or(miette::miette!("No package by name: {package_name}"))?;

        let version = package.write_cargo_file()?;
        tracing::info!("Written '{version}' to {package_name}");
        Ok(())
    }
}
