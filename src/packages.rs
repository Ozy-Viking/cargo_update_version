#![allow(dead_code)]
use std::{collections::HashMap, fmt::Display, ops::DerefMut, path::PathBuf};

use cargo_metadata::Metadata;
use semver::Version;
use tracing::{debug, instrument};

/// Newtype around Package Name.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct PackageName(String);

impl Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<T> From<T> for PackageName
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl std::ops::Deref for PackageName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PackageName {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsMut<str> for PackageName {
    fn as_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Packages {
    /// File path to the root cargo.toml
    cargo_file_path: PathBuf,
    /// Root package of the rust project.
    root_package: Option<PackageName>,
    /// Hashmap of the packages in the rust project.
    packages: HashMap<PackageName, Package>,
}

impl Packages {
    pub fn new<P>(path: Option<PathBuf>, packages: &[P], root_package: Option<&P>) -> Self
    where
        P: Into<Package> + Clone,
    {
        let mut ret = Self {
            cargo_file_path: path.unwrap_or_default(),
            root_package: None,
            packages: HashMap::from_iter(packages.iter().map(|package| {
                let package: Package = package.clone().into();
                (package.name.clone(), package)
            })),
        };
        if let Some(root_package) = root_package {
            ret.root_package = ret
                .set_root_package_name(root_package.clone().into().name())
                .cloned();
        }

        ret
    }

    #[instrument()]
    pub fn cargo_file_path(&self) -> &PathBuf {
        debug!("Fetching cargo path");
        &self.cargo_file_path
    }
    pub fn drop_root_package_name(&mut self) {
        self.root_package = None
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
    pub fn get_root_package(&self) -> Option<&Package> {
        let root_name = self.root_package_name_unchecked()?;
        self.packages.get(root_name)
    }

    /// Checks whether there is a root name and returns a mut ref to the [root_package][Package].
    pub fn get_root_package_mut(&mut self) -> Option<&mut Package> {
        let root_name = self.root_package_name_unchecked()?.clone();
        self.packages.get_mut(&root_name)
    }

    /// Checks whether there is a root name and returns an owned clone of the [root_package][Package].
    pub fn get_root_package_owned(&self) -> Option<Package> {
        let root_name = self.root_package_name_unchecked()?;
        self.packages.get(root_name).cloned()
    }

    /// Uses the [HashMap::get] and returns a ref to the [Package] if it exists.
    pub fn get_package(&self, package_name: &PackageName) -> Option<&Package> {
        self.packages.get(package_name)
    }

    ///  Uses the [HashMap::get_mut] and returns a mut ref to the [Package] if it exists.
    pub fn get_package_mut(&mut self, package_name: &PackageName) -> Option<&mut Package> {
        self.packages.get_mut(package_name)
    }

    ///  Uses the [HashMap::get] followed by [Option::cloned] and returns [Package] if it exists.
    pub fn get_package_owned(&self, package_name: &PackageName) -> Option<Package> {
        self.packages.get(package_name).cloned()
    }

    pub fn packages(&self) -> &HashMap<PackageName, Package> {
        &self.packages
    }

    pub fn set_cargo_file_path(&mut self, cargo_file: PathBuf) {
        self.cargo_file_path = cargo_file;
    }

    pub(crate) fn get_root_package_version(&self) -> Option<Version> {
        self.get_root_package().map(|rp| rp.version.clone())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct Package {
    name: PackageName,
    version: Version,
}

impl Package {
    pub fn name(&self) -> &PackageName {
        &self.name
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.version
    }
}

impl From<cargo_metadata::Package> for Package {
    fn from(meta_package: cargo_metadata::Package) -> Self {
        Self {
            name: meta_package.name.to_string().into(),
            version: meta_package.version,
        }
    }
}

impl From<&Metadata> for Packages {
    fn from(metadata: &Metadata) -> Self {
        Self::new(None, &metadata.packages, metadata.root_package())
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[diagnostic(url(docsrs))]
pub enum PackageError {
    #[error("Package ({0}) not found in Cargo.toml")]
    #[diagnostic(code(PackageError::PackageNameNotFound))]
    PackageNameNotFound(PackageName),
    #[error("Package name not provided")]
    #[diagnostic(code(PackageError::PackageNameNotProvided))]
    PackageNameNotProvided,
}
