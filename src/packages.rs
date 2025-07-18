#![allow(dead_code)]

mod package_name;
pub use package_name::PackageName;

use std::io::Write;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use cargo_metadata::Metadata;
use semver::Version;
use tracing::{debug, instrument};

#[derive(Debug, Clone, PartialEq)]
pub struct Packages {
    /// File path to the root cargo.toml
    root_manifest_path: PathBuf,
    /// Root package of the rust project.
    root_package: Option<PackageName>,

    /// Default members
    default_members: HashSet<PackageName>,

    /// Hashmap of the packages in the rust project.
    packages: HashMap<PackageName, Package>,
}

impl AsRef<HashMap<PackageName, Package>> for Packages {
    fn as_ref(&self) -> &HashMap<PackageName, Package> {
        &self.packages
    }
}

impl Packages {
    pub fn new<P, N>(
        path: PathBuf,
        packages: &[P],
        root_package: Option<&P>,
        default_members: Vec<N>,
    ) -> Self
    where
        P: Into<Package> + Clone,
        N: ToString + Clone,
    {
        let mut ret = Self {
            root_manifest_path: path,
            root_package: None,
            packages: HashMap::from_iter(packages.iter().map(|package| {
                let package: Package = package.clone().into();
                (package.name.clone(), package)
            })),
            default_members: HashSet::from_iter(
                default_members.iter().map(|n| PackageName(n.to_string())),
            ),
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

    pub fn package_set(&self) -> HashSet<&Package> {
        self.packages.values().collect::<HashSet<_>>()
    }

    pub fn set_cargo_file_path(&mut self, cargo_file: PathBuf) {
        self.root_manifest_path = cargo_file;
    }

    pub(crate) fn get_root_package_version(&self) -> Option<Version> {
        self.get_root_package().map(|rp| rp.version.clone())
    }
}

impl Packages {
    pub fn display_tree(&self) -> String {
        let mut ret_string = Vec::new();
        let root_package = self.root_package.as_ref();
        let path_base = self.cargo_file_path().parent().unwrap();
        let make_relative = |package: &Package| {
            PathBuf::new()
                .join(".")
                .join(
                    package
                        .manifest_path
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
            let _ = writeln!(ret_string, "Root package: {root} {}", package.version,);
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
                    package.version,
                    make_relative(package)
                );
            } else {
                let _ = writeln!(
                    ret_string,
                    "├─ {name} {}: {}",
                    package.version,
                    make_relative(package)
                );
            }
        }

        // root package: a
        // default members: ["a"]
        //
        // All members:
        //     ├─ "a"
        //     └─ "b"

        String::from_utf8(ret_string).expect("Chars is valid utf-8")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct Package {
    name: PackageName,
    version: Version,
    manifest_path: PathBuf,
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
            manifest_path: meta_package.manifest_path.into(),
        }
    }
}

impl From<&Metadata> for Packages {
    fn from(metadata: &Metadata) -> Self {
        let root_path = metadata.workspace_root.join("Cargo.toml");

        let default_members = metadata
            .workspace_default_packages()
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();

        Self::new(
            root_path.into(),
            &metadata.packages,
            metadata.root_package(),
            default_members,
        )
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
