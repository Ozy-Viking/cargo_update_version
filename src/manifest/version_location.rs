use std::{fmt::Display, panic::Location, path::PathBuf};

use rusty_viking::EnumDisplay;
use semver::Version;
use tracing::{info, instrument, trace};

use crate::{CargoFile, ReadToml, current_span};

#[derive(Debug, Clone, Copy)]
pub enum VersionLocation {
    Package,
    WorkspacePackage,
}

impl Display for VersionLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                VersionLocation::Package => "package.version",
                VersionLocation::WorkspacePackage => "workspace.package.version",
            }
        )
    }
}

impl VersionLocation {
    #[track_caller]
    #[instrument(skip_all, fields(version, path))]
    pub fn get_version<'a>(
        &self,
        cargo_toml: &CargoFile<'a, ReadToml>,
    ) -> Result<Version, VersionlocationError> {
        use VersionLocationErrorKind as ErrKind;
        let path = cargo_toml.path();

        let set_err = |kind: VersionLocationErrorKind, context: Option<&'static str>| {
            kind.to_error(path, context)
        };
        let _span = current_span!();
        let document = cargo_toml
            .contents()
            .expect("Can't call this function without the document read.");
        trace!("have document");
        let ret = match self {
            VersionLocation::Package => {
                let package = document
                    .get("package")
                    .ok_or(set_err(ErrKind::PackageNotFound, None))?;

                let package_table = package.as_table().ok_or_else(|| {
                    set_err(
                        ErrKind::ItemInvalid(package.into()),
                        Some("Tried to get package table"),
                    )
                })?;

                match package_table.get("version").ok_or(set_err(
                    ErrKind::NotFound(*self),
                    Some("Package table located."),
                ))? {
                    toml_edit::Item::Value(value) => {
                        Version::parse(value.as_str().unwrap()).map_err(|e| set_err(e.into(), None))
                    }
                    toml_edit::Item::Table(table) => {
                        let workspace_table = table.get("workspace").ok_or(set_err(
                            ErrKind::ItemInvalid(ItemType::Table),
                            Some("expected version.workspace = <bool>"),
                        ))?;
                        let val = workspace_table.as_bool().ok_or(set_err(
                            ErrKind::ItemInvalid(ItemType::Value),
                            Some("Expected bool"),
                        ))?;

                        let msg = format!("in the manifest file: version.workspace = {val}");
                        Err(ErrKind::SetByWorkspace.to_error(path, Some(msg)))
                    }

                    item => Err(set_err(ErrKind::ItemInvalid(item.into()), None)),
                }
            }
            VersionLocation::WorkspacePackage => {
                let workspace = document
                    .get("workspace")
                    .ok_or(set_err(ErrKind::WorkspaceNotFound, None))?;

                let workspace = workspace.as_table().ok_or_else(|| {
                    set_err(
                        ErrKind::ItemInvalid(workspace.into()),
                        Some("Tried to get package table"),
                    )
                })?;
                let package = workspace
                    .get("package")
                    .ok_or(set_err(ErrKind::PackageNotFound, None))?;

                let package = package.as_table().ok_or_else(|| {
                    set_err(
                        ErrKind::ItemInvalid(package.into()),
                        Some("Tried to get package table"),
                    )
                })?;
                match package.get("version").ok_or(set_err(
                    ErrKind::NotFound(*self),
                    Some("Package table located."),
                ))? {
                    toml_edit::Item::Value(value) => Version::parse(value.as_str().unwrap())
                        .map_err(|e| set_err(e.into(), Some("Workspace Version"))),
                    item => Err(set_err(ErrKind::ItemInvalid(item.into()), None)),
                }
            }
        };
        let version = ret?;
        current_span!().record("path", path.as_os_str().to_str().unwrap_or_default());
        current_span!().record("version", &version.to_string());
        info!("Version found");

        Ok(version)
    }

    #[track_caller]
    #[instrument(skip_all, fields(version, path))]
    pub fn set_version<'a>(
        &self,
        cargo_toml: &mut CargoFile<'a, ReadToml>,
        version: &Version,
    ) -> Result<(), VersionlocationError> {
        use VersionLocationErrorKind as ErrKind;
        let path = cargo_toml.path().to_path_buf();

        let set_err = |kind: VersionLocationErrorKind, context: Option<&'static str>| {
            kind.to_error(&path, context)
        };

        let _span = current_span!();

        let document = cargo_toml
            .contents_mut()
            .expect("Can't call this function without the document read.");

        trace!("have document");

        let ret = match self {
            VersionLocation::Package => {
                let package = document
                    .get_mut("package")
                    .ok_or(set_err(ErrKind::PackageNotFound, None))?;
                let pack_kind = ItemType::from(&*package);

                let package_table = package.as_table_mut().ok_or_else(|| {
                    set_err(
                        ErrKind::ItemInvalid(pack_kind),
                        Some("Tried to get package table"),
                    )
                })?;

                match package_table.get_mut("version").ok_or(set_err(
                    ErrKind::NotFound(*self),
                    Some("Package table located."),
                ))? {
                    toml_edit::Item::Value(value) => Ok(*value = version.to_string().into()),
                    item => Err(set_err(
                        ErrKind::ItemInvalid(item.into()),
                        Some("Invalid itemtype for setting package version."),
                    )),
                }
            }
            VersionLocation::WorkspacePackage => {
                let workspace = document
                    .get_mut("workspace")
                    .ok_or(set_err(ErrKind::WorkspaceNotFound, None))?;

                let ws_kind = ItemType::from(&*workspace);
                let workspace = workspace.as_table_mut().ok_or_else(|| {
                    set_err(
                        ErrKind::ItemInvalid(ws_kind.into()),
                        Some("Tried to get package table"),
                    )
                })?;
                let package = workspace
                    .get_mut("package")
                    .ok_or(set_err(ErrKind::PackageNotFound, None))?;
                let pack_kind = ItemType::from(&*package);

                let package = package.as_table_mut().ok_or_else(|| {
                    set_err(
                        ErrKind::ItemInvalid(pack_kind.into()),
                        Some("Tried to get package table"),
                    )
                })?;
                match package.get_mut("version").ok_or(set_err(
                    ErrKind::NotFound(*self),
                    Some("Package table located."),
                ))? {
                    toml_edit::Item::Value(value) => Ok(*value = version.to_string().into()),
                    item => Err(set_err(
                        ErrKind::ItemInvalid(item.into()),
                        Some("Invalid itemtype for setting workspace version."),
                    )),
                }
            }
        };
        let _version = ret?;
        current_span!().record("path", (&path).as_os_str().to_str().unwrap_or_default());
        info!("Version set: {version}");
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum VersionLocationErrorKind {
    #[error("Version is set by workspace.")]
    SetByWorkspace,
    #[error("Version not located in {0}")]
    NotFound(VersionLocation),
    #[error("No package defined in toml file.")]
    PackageNotFound,
    #[error("No workspace defined in toml file.")]
    WorkspaceNotFound,
    #[error("Invalid item type of {0}")]
    ItemInvalid(ItemType),
    #[error("Invalid semver: {0}")]
    SemverError(semver::Error),
}

impl From<semver::Error> for VersionLocationErrorKind {
    fn from(value: semver::Error) -> Self {
        Self::SemverError(value)
    }
}

impl VersionLocationErrorKind {
    #[track_caller]
    pub fn to_error(
        self,
        path: impl Into<PathBuf>,
        context: Option<impl ToString + std::default::Default>,
    ) -> VersionlocationError {
        VersionlocationError {
            kind: self,
            path: path.into(),
            location_called: Location::caller(),
            context: context.unwrap_or_default().to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("context")]
pub struct VersionlocationError {
    #[source]
    kind: VersionLocationErrorKind,
    path: PathBuf,
    #[help("Call location: {location_called}")]
    location_called: &'static Location<'static>,
    context: String,
}

impl VersionlocationError {
    pub fn new(
        kind: VersionLocationErrorKind,
        path: PathBuf,
        location_called: &'static Location<'static>,
        context: String,
    ) -> Self {
        Self {
            kind,
            path,
            location_called,
            context,
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn kind(&self) -> &VersionLocationErrorKind {
        &self.kind
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, EnumDisplay)]
#[Title]
pub enum ItemType {
    None,
    Value,
    Table,
    ArrayOfTables,
}

impl From<toml_edit::Item> for ItemType {
    fn from(item: toml_edit::Item) -> Self {
        match item {
            toml_edit::Item::None => ItemType::None,
            toml_edit::Item::Value(_) => ItemType::Value,
            toml_edit::Item::Table(_) => ItemType::Table,
            toml_edit::Item::ArrayOfTables(_) => ItemType::ArrayOfTables,
        }
    }
}
impl From<&toml_edit::Item> for ItemType {
    fn from(item: &toml_edit::Item) -> Self {
        match item {
            toml_edit::Item::None => ItemType::None,
            toml_edit::Item::Value(_) => ItemType::Value,
            toml_edit::Item::Table(_) => ItemType::Table,
            toml_edit::Item::ArrayOfTables(_) => ItemType::ArrayOfTables,
        }
    }
}
impl From<&mut toml_edit::Item> for ItemType {
    fn from(item: &mut toml_edit::Item) -> Self {
        match item {
            toml_edit::Item::None => ItemType::None,
            toml_edit::Item::Value(_) => ItemType::Value,
            toml_edit::Item::Table(_) => ItemType::Table,
            toml_edit::Item::ArrayOfTables(_) => ItemType::ArrayOfTables,
        }
    }
}
