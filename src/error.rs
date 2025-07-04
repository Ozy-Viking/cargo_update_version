use std::fmt::{self, Display, Formatter};

use miette::Diagnostic;
use semver::Version;

use crate::Action;

#[allow(dead_code)]
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum UvError {
    #[error("{msg}")]
    Clap {
        kind: clap::error::ErrorKind,
        msg: String,
        #[help]
        help: String,
        #[source_code]
        source_code: String,
        #[label("{label_msg}")]
        label: Option<(usize, usize)>,
        label_msg: &'static str,
    },
    #[error("{msg}")]
    Semver {
        msg: String,
        source_code: String,
        help: String,
    },
    #[error(transparent)]
    ManifestNotFound(ManifestNotFoundError),
    #[error("Your guess is as good as mine.")]
    Unknown,
}

#[derive(Debug, Clone, thiserror::Error, miette::Diagnostic)]
pub struct ManifestNotFoundError {
    #[help]
    pub help: Option<&'static str>,
    #[source_code]
    pub source_code: Option<String>,
    pub msg: String,
    #[label("{label_msg}")]
    pub label: Option<(usize, usize)>,
    pub label_msg: &'static str, // source: Option<&'static str>,
}

impl Display for ManifestNotFoundError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<ManifestNotFoundError> for UvError {
    fn from(value: ManifestNotFoundError) -> Self {
        Self::ManifestNotFound(value)
    }
}

#[derive(Debug, thiserror::Error, Diagnostic)]
pub struct VersionError {
    pub old_version: semver::Version,
    pub bump: Action,
    pub msg: String,
    #[help]
    pub help: Option<String>,
    #[label("{label_msg}")]
    pub label: Option<(usize, usize)>,
    pub label_msg: String,
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl VersionError {
    pub fn prerelease_not_empty(old_version: &Version, bump: Action) -> Self {
        let msg = "Pre-release is not empty.".to_string();
        // TODO: Fix the wording for version error.
        let help = Some(format!(
            "To version bump by {}, pre-release needs to be empty. Use '--force-version' to skip this check.",
            bump
        ));

        Self {
            old_version: old_version.clone(),
            bump,
            msg,
            help,
            label: None,
            label_msg: "".into(),
        }
    }
}
