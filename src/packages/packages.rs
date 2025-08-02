use std::io::Write;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use cargo_metadata::Metadata;
use semver::Version;
use tracing::{debug, instrument};

use super::{Package, PackageError, PackageName};
use crate::{ReadToml, Result, VersionLocation, display_path};

#[derive(Debug, Clone, PartialEq)]
pub struct Packages {
    /// File path to the root cargo.toml
    root_manifest_path: PathBuf,
    /// Root package of the rust project.
    root_package: Option<PackageName>,

    /// The root package version or if not present the workplace version.
    root_version: Option<Version>,

    /// Default members
    default_members: HashSet<PackageName>,

    /// Hashmap of the packages in the rust project.
    packages: HashMap<PackageName, Package<ReadToml>>,

    /// The package of the root Cargo.toml
    workspace_package: Option<Package<ReadToml>>,
}

impl AsRef<HashMap<PackageName, Package<ReadToml>>> for Packages {
    fn as_ref(&self) -> &HashMap<PackageName, Package<ReadToml>> {
        &self.packages
    }
}

impl Packages {
    pub fn new<P, N>(
        path: PathBuf,
        packages: &[P],
        root_package: Option<&P>,
        default_members: Vec<N>,
    ) -> Result<Self>
    where
        P: Into<Package<ReadToml>> + Clone,
        N: ToString + Clone,
    {
        let workspace_package = Package::workspace_package(&path).ok();
        let mut ret = Self {
            root_manifest_path: path,
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
        };
        if let Some(root_package) = root_package {
            let root_package: Package<ReadToml> = root_package.clone().into();
            ret.root_package = ret.set_root_package_name(root_package.name()).cloned();
            ret.root_version = Some(root_package.version().clone());
        }
        Ok(ret)
    }

    #[instrument()]
    pub fn cargo_file_path(&self) -> &PathBuf {
        debug!("Fetching cargo path");
        &self.root_manifest_path
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
        self.packages.get(package_name)
    }

    ///  Uses the [HashMap::get_mut] and returns a mut ref to the [Package] if it exists.
    pub fn get_package_mut(
        &mut self,
        package_name: &PackageName,
    ) -> Option<&mut Package<ReadToml>> {
        self.packages.get_mut(package_name)
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

    pub fn set_cargo_file_path(&mut self, cargo_file: PathBuf) {
        self.root_manifest_path = cargo_file;
    }

    pub(crate) fn get_root_package_version(&self) -> Option<Version> {
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

        if let Some(workspace_package) = &self.workspace_package {
            // Checking the workspace package
            match VersionLocation::WorkspacePackage.get_version(workspace_package.cargo_file()) {
                Ok(v) => return Ok(v),
                Err(e) => match e.kind() {
                    crate::VersionLocationErrorKind::SetByWorkspace => (),
                    crate::VersionLocationErrorKind::NotFound(_) => (),
                    crate::VersionLocationErrorKind::PackageNotFound => unreachable!(),
                    crate::VersionLocationErrorKind::WorkspaceNotFound => {
                        todo!("Workspace Not Found")
                    }
                    crate::VersionLocationErrorKind::ItemInvalid(_) => (),
                    crate::VersionLocationErrorKind::SemverError(_) => (),
                },
            };
        }

        Err(error_no_root_package)
    }

    pub fn workspace_package(&mut self) -> Option<&mut Package<ReadToml>> {
        self.workspace_package.as_mut()
    }
}

impl Packages {
    pub fn display_tree(&self) -> String {
        let mut ret_string = Vec::new();
        let root_package = self.root_package.as_ref();
        let path_base = self.cargo_file_path().parent().unwrap();
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
            root_path.into(),
            &metadata.packages,
            metadata.root_package(),
            default_members,
        )
        .expect("From cargo metadata")
    }
}
