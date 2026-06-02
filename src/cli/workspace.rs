//! Replica of crate: [clap-cargo](https://github.com/crate-ci/clap-cargo)
//! Cargo flags for selecting crates in a workspace.

use std::collections::HashSet;

use tracing::{instrument, trace};

use crate::{Package, PackageName, Packages, ReadToml, Result, SplitVec, cli::WORKSPACE_HEADER};

/// Cargo flags for selecting crates in a workspace.
#[derive(Default, Clone, Debug, PartialEq, Eq, clap::Args)]
#[command(about = None, long_about = None)]
#[non_exhaustive]
pub struct Workspace {
    #[arg(short, long, value_name = "SPEC", help_heading = WORKSPACE_HEADER)]
    /// Package to process (see `cargo help pkgid`)
    pub package: Vec<String>,

    #[arg(short = 'x', long, value_name = "SPEC", help_heading = WORKSPACE_HEADER)]
    /// Exclude packages from being processed
    pub exclude: Vec<String>,

    #[arg(long, visible_alias("all"), help_heading = WORKSPACE_HEADER, conflicts_with("default_members") )]
    /// Process all packages in the workspace
    pub workspace: bool,

    #[arg(long, visible_alias("ws"), help_heading = WORKSPACE_HEADER)]
    /// Process workspace.package.version
    pub workspace_package: bool,

    #[arg(long, help_heading = WORKSPACE_HEADER, conflicts_with("workspace"))]
    /// Process only default workspace members
    pub default_members: bool,
}

impl Workspace {
    #[instrument(skip(packages))]
    /// Partition workspace members into those selected and those excluded.
    ///
    /// Notes:
    /// - Requires the features `cargo_metadata`.
    /// - Requires not calling `MetadataCommand::no_deps`
    pub fn partition_packages<'m>(
        &self,
        packages: &'m Packages,
    ) -> Result<SplitVec<&'m Package<ReadToml>>> {
        let selection = PackagesCli::from_flags(
            self.workspace,
            self.default_members,
            &self.exclude,
            &self.package,
        );
        let root_package = packages.root_package_name_unchecked();
        let modifications: &PackagesCliModifier<'_> = selection.as_ref();
        let workspace_members: HashSet<&PackageName> = packages.workspace_members();
        let workspace_default_members: HashSet<&PackageName> = packages.workspace_default_members();

        let base_ids: HashSet<&PackageName> = match selection {
            PackagesCli::RootPackage(_) => workspace_members
                .iter()
                .filter_map(|&p| (Some(p) == root_package).then_some(p))
                .collect(),
            PackagesCli::All(_) => workspace_members,
            PackagesCli::DefaultMembers(_) => workspace_default_members,
        };

        Ok(packages
            .package_set()
            .into_iter()
            // Deviating from cargo by not supporting patterns
            .partition(|package| modifications.include(&base_ids, package.name())))
    }

    pub fn partition_packages_owned(
        &self,
        packages: &Packages,
    ) -> Result<SplitVec<Package<ReadToml>>> {
        self.partition_packages(packages).map(|(i, e)| {
            (
                i.into_iter().cloned().collect(),
                e.into_iter().cloned().collect(),
            )
        })
    }

    pub fn partition_packages_mut<'m>(
        &self,
        packages: &'m mut Packages,
    ) -> Result<SplitVec<&'m mut Package<ReadToml>>> {
        let packages_clone = packages.clone();
        let selection = PackagesCli::from_flags(
            self.workspace,
            self.default_members,
            &self.exclude,
            &self.package,
        );
        let root_package = packages.root_package_name();
        let modifications: &PackagesCliModifier<'_> = selection.as_ref();
        let workspace_members = packages_clone.workspace_members();
        let workspace_default_members: HashSet<&PackageName> =
            packages_clone.workspace_default_members();

        let base_ids = match selection {
            PackagesCli::RootPackage(_) => workspace_members
                .iter()
                .filter_map(|&p| (Some(p) == root_package).then_some(p))
                .collect(),
            PackagesCli::All(_) => workspace_members,
            PackagesCli::DefaultMembers(_) => workspace_default_members,
        };

        Ok(packages
            .package_set_mut()
            .into_iter()
            // Deviating from cargo by not supporting patterns
            .partition(|package| modifications.include(&base_ids, package.name())))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PackagesCli<'p> {
    RootPackage(PackagesCliModifier<'p>),
    All(PackagesCliModifier<'p>),
    DefaultMembers(PackagesCliModifier<'p>),
}

impl<'p> AsRef<PackagesCliModifier<'p>> for PackagesCli<'p> {
    fn as_ref(&self) -> &PackagesCliModifier<'p> {
        match self {
            PackagesCli::RootPackage(packages_cli_modifier) => packages_cli_modifier,
            PackagesCli::All(packages_cli_modifier) => packages_cli_modifier,
            PackagesCli::DefaultMembers(packages_cli_modifier) => packages_cli_modifier,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PackagesCliModifier<'p> {
    include: Option<&'p [String]>,
    exclude: Option<&'p [String]>,
}

impl<'p> PackagesCliModifier<'p> {
    const NO_MOD: Self = PackagesCliModifier {
        include: None,
        exclude: None,
    };

    pub fn new(mut include: Option<&'p [String]>, mut exclude: Option<&'p [String]>) -> Self {
        if let Some(inc) = include {
            if inc.is_empty() {
                include = None;
            }
        }
        if let Some(ex) = exclude {
            if ex.is_empty() {
                exclude = None;
            }
        }
        Self { include, exclude }
    }

    /// Tests whether to include the package, uses both included and excluded.
    pub fn include(&self, base_ids: &HashSet<&PackageName>, package: &String) -> bool {
        let is_include = if let Some(inc) = self.include {
            inc.contains(package)
        } else {
            false
        };
        let is_excluded = self.exclude(package);

        match (base_ids.contains(package), is_include, is_excluded) {
            (false, false, _) => false,
            (_, _, false) => true,
            (_, _, true) => false,
        }
    }

    /// Test whether the package has been explicitly excluded.
    pub fn exclude(&self, package: &String) -> bool {
        if let Some(exc) = self.exclude {
            exc.contains(package)
        } else {
            false
        }
    }
}

impl<'p> PackagesCli<'p> {
    #[instrument]
    pub fn from_flags(
        all: bool,
        default_members: bool,
        exclude: &'p [String],
        package: &'p [String],
    ) -> Self {
        trace!("from_flags");
        use PackagesCliModifier as PackMod;
        let pack_mod = PackMod::new(Some(package), Some(exclude));
        match (all, default_members, exclude.len(), package.len()) {
            (false, false, 0, 0) => PackagesCli::RootPackage(PackMod::NO_MOD),
            (true, false, 0, _) => PackagesCli::All(PackMod::NO_MOD),
            (true, false, _, _) => PackagesCli::All(pack_mod),
            (false, true, _, _) => PackagesCli::DefaultMembers(pack_mod),
            (false, false, _, _) => PackagesCli::RootPackage(pack_mod),
            (true, true, _, _) => unreachable!(),
        }
    }
}
#[cfg(test)]
mod tests {
    use cargo_metadata::MetadataCommand;

    use super::*;
    use crate::Packages;

    fn fixture(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative)
    }

    fn packages_from(manifest: &str) -> Packages {
        let metadata = MetadataCommand::new()
            .manifest_path(fixture(manifest))
            .exec()
            .unwrap();
        Packages::from(&metadata)
    }

    #[test]
    fn verify_app() {
        #[derive(Debug, clap::Parser)]
        struct Cli {
            #[command(flatten)]
            workspace: Workspace,
        }
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_flags() {
        use clap::Parser;

        #[derive(PartialEq, Eq, Debug, Parser)]
        struct Args {
            positional: Option<String>,
            #[command(flatten)]
            workspace: Workspace,
        }

        assert_eq!(
            Args::parse_from(["test"]),
            Args {
                positional: None,
                workspace: Workspace::default(),
            }
        );
        assert_eq!(
            Args::parse_from(["test", "--package", "foo", "--package", "bar", "baz"]),
            Args {
                positional: Some("baz".to_owned()),
                workspace: Workspace {
                    package: vec!["foo".to_owned(), "bar".to_owned()],
                    ..Default::default()
                },
            }
        );
        assert_eq!(
            Args::parse_from(["test", "--exclude", "foo", "--exclude", "bar", "baz"]),
            Args {
                positional: Some("baz".to_owned()),
                workspace: Workspace {
                    exclude: vec!["foo".to_owned(), "bar".to_owned()],
                    ..Default::default()
                },
            }
        );
        assert_eq!(
            Args::parse_from(["test", "--workspace"]),
            Args {
                positional: None,
                workspace: Workspace {
                    workspace: true,
                    ..Default::default()
                },
            }
        );
    }

    mod partition_default {
        use super::*;

        #[test]
        fn single_crate() {
            let packages = packages_from("simple/Cargo.toml");
            let (included, excluded) = Workspace::default()
                .partition_packages(&packages)
                .unwrap();
            assert_eq!(included.len(), 1);
            assert_eq!(excluded.len(), 0);
        }

        #[test]
        fn mixed_ws_from_root() {
            let packages = packages_from("mixed_ws/Cargo.toml");
            let (included, excluded) = Workspace::default()
                .partition_packages(&packages)
                .unwrap();
            // default selects only the workspace root package
            assert_eq!(included.len(), 1);
            assert_eq!(excluded.len(), 2);
        }

        #[test]
        fn mixed_ws_from_leaf() {
            // cargo metadata resolves back to workspace root regardless of entry manifest
            let packages = packages_from("mixed_ws/c/Cargo.toml");
            let (included, excluded) = Workspace::default()
                .partition_packages(&packages)
                .unwrap();
            assert_eq!(included.len(), 1);
            assert_eq!(excluded.len(), 2);
        }

        #[test]
        fn pure_ws_from_root() {
            let packages = packages_from("pure_ws/Cargo.toml");
            let (included, excluded) = Workspace::default()
                .partition_packages(&packages)
                .unwrap();
            // virtual workspace: no root package → nothing selected by default
            assert_eq!(included.len(), 0);
            assert_eq!(excluded.len(), 3);
        }

        #[test]
        fn pure_ws_from_leaf() {
            // When invoked from a leaf manifest, cargo resolves that leaf as the root package
            let packages = packages_from("pure_ws/c/Cargo.toml");
            let (included, excluded) = Workspace::default()
                .partition_packages(&packages)
                .unwrap();
            assert_eq!(included.len(), 1); // c is the cargo resolve root
            assert_eq!(excluded.len(), 2);
        }
    }

    mod partition_all {
        use super::*;

        fn all_workspace() -> Workspace {
            Workspace {
                workspace: true,
                ..Default::default()
            }
        }

        #[test]
        fn single_crate() {
            let packages = packages_from("simple/Cargo.toml");
            let (included, excluded) = all_workspace().partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 1);
            assert_eq!(excluded.len(), 0);
        }

        #[test]
        fn mixed_ws_from_root() {
            let packages = packages_from("mixed_ws/Cargo.toml");
            let (included, excluded) = all_workspace().partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 3);
            assert_eq!(excluded.len(), 0);
        }

        #[test]
        fn mixed_ws_from_leaf() {
            let packages = packages_from("mixed_ws/c/Cargo.toml");
            let (included, excluded) = all_workspace().partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 3);
            assert_eq!(excluded.len(), 0);
        }

        #[test]
        fn pure_ws_from_root() {
            let packages = packages_from("pure_ws/Cargo.toml");
            let (included, excluded) = all_workspace().partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 3);
            assert_eq!(excluded.len(), 0);
        }

        #[test]
        fn pure_ws_from_leaf() {
            let packages = packages_from("pure_ws/c/Cargo.toml");
            let (included, excluded) = all_workspace().partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 3);
            assert_eq!(excluded.len(), 0);
        }
    }

    mod partition_package {
        use super::*;

        #[test]
        fn single_crate_explicit() {
            let packages = packages_from("simple/Cargo.toml");
            let ws = Workspace {
                package: vec!["simple".to_owned()],
                ..Default::default()
            };
            let (included, excluded) = ws.partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 1);
            assert_eq!(excluded.len(), 0);
        }

        #[test]
        fn mixed_ws_explicit_includes_root() {
            // In mixed_ws, -p a selects a plus the workspace root (b stays in base set)
            let packages = packages_from("mixed_ws/Cargo.toml");
            let ws = Workspace {
                package: vec!["a".to_owned()],
                ..Default::default()
            };
            let (included, excluded) = ws.partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 2); // a + root (b)
            assert_eq!(excluded.len(), 1); // c
        }

        #[test]
        fn pure_ws_explicit_only() {
            // No root in pure_ws, so -p a selects only a
            let packages = packages_from("pure_ws/Cargo.toml");
            let ws = Workspace {
                package: vec!["a".to_owned()],
                ..Default::default()
            };
            let (included, excluded) = ws.partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 1); // only a
            assert_eq!(excluded.len(), 2); // b, c
        }
    }

    mod partition_exclude {
        use super::*;

        #[test]
        fn mixed_ws_exclude_one() {
            let packages = packages_from("mixed_ws/Cargo.toml");
            let ws = Workspace {
                workspace: true,
                exclude: vec!["a".to_owned()],
                ..Default::default()
            };
            let (included, excluded) = ws.partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 2); // b, c
            assert_eq!(excluded.len(), 1); // a
        }

        #[test]
        fn pure_ws_exclude_one() {
            let packages = packages_from("pure_ws/Cargo.toml");
            let ws = Workspace {
                workspace: true,
                exclude: vec!["b".to_owned()],
                ..Default::default()
            };
            let (included, excluded) = ws.partition_packages(&packages).unwrap();
            assert_eq!(included.len(), 2); // a, c
            assert_eq!(excluded.len(), 1); // b
        }
    }
}
