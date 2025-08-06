use std::{
    fmt::{self, Display, Formatter},
    panic::Location,
    path::PathBuf,
};

use miette::Diagnostic;
use rusty_viking::EnumDisplay;

use crate::VersionLocation;

#[derive(Debug, thiserror::Error, Diagnostic)]
pub struct CargoFileError {
    #[source]
    kind: CargoFileErrorKind,
    path: PathBuf,
    #[help("Call location: {location_called}")]
    location_called: Location<'static>,
}

impl CargoFileError {
    pub fn kind(&self) -> &CargoFileErrorKind {
        &self.kind
    }
}

impl Display for CargoFileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Cargo.toml: {}", self.path.display())
    }
}

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
    pub fn to_error(self, path: impl Into<PathBuf>) -> CargoFileError {
        CargoFileError {
            kind: self,
            path: path.into(),
            location_called: *Location::caller(),
        }
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
#[error("{context}")]
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
