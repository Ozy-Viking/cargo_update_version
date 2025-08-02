use std::{fmt::Display, process::Child};

use semver::{BuildMetadata, Prerelease, Version};

use crate::{Action, Package, ReadToml, Result};

#[derive(Hash, PartialEq, Debug, Eq, Clone)]
pub enum Task {
    Push(String),
    Publish,
    Print,
    Tree,
    Set {
        version: Version,
        package: Package<ReadToml>,
    },
    Bump {
        package: Package<ReadToml>,
        bump: Action,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force: bool,
    },
    BumpWorkspace {
        bump: Action,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force: bool,
    },
    SetWorkspace {
        version: Version,
    },
    DeleteGitTag(Version),
    ChangeBranch {
        to: String,
        from: String,
    },
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Task::Push(remote) => &format!("Push to {remote}"),
            Task::Publish => "Publish",
            Task::Print => "Print",
            Task::Tree => "Tree",
            Task::Set { version, package } => {
                &format!("Set {}: {}", package.name(), version.to_string())
            }
            Task::Bump { package, bump, .. } => &format!("Bump {bump}: {}", package.name()),
            Task::BumpWorkspace { bump, .. } => &format!("Bump Workspace Package: {}", bump),
            Task::SetWorkspace { version } => &format!("Set Workspace: {}", version.to_string()),
            Task::DeleteGitTag(version) => &format!("Delete Git Tag: {}", version.to_string()),
            Task::ChangeBranch { to, .. } => &format!("Change branch: {}", to),
        };
        write!(f, "{}", text)
    }
}

impl Task {
    pub fn is_version_change(&self) -> bool {
        match self {
            Task::ChangeBranch { .. }
            | Task::Push(_)
            | Task::Publish
            | Task::Print
            | Task::DeleteGitTag(_)
            | Task::Tree => false,

            Task::Set { .. }
            | Task::Bump { .. }
            | Task::BumpWorkspace { .. }
            | Task::SetWorkspace { .. } => true,
        }
    }

    /// Returns `true` if the task is [`DeleteGitTag`].
    ///
    /// [`DeleteGitTag`]: Task::DeleteGitTag
    #[must_use]
    pub fn is_delete_git_tag(&self) -> bool {
        matches!(self, Self::DeleteGitTag(..))
    }
}

/// TODO: Make a reference.
impl Task {
    pub fn from_action(
        action: Action,
        package: Package<ReadToml>,
        pre: Option<Prerelease>,
        build: Option<BuildMetadata>,
        new_version: Option<Version>,
        force: bool,
    ) -> Result<Task> {
        match action {
            Action::Pre | Action::Patch | Action::Minor | Action::Major => Ok(Task::Bump {
                package: package,
                bump: action,
                pre,
                build,
                force,
            }),
            Action::Set => Ok(Task::Set {
                version: new_version.ok_or(miette::miette!(
                    "Expected a new version for Task::from_action when action is Set"
                ))?,
                package,
            }),
            Action::Print => Ok(Task::Tree),
            Action::Tree => Ok(Task::Print),
        }
    }
}

impl Task {
    pub fn run(&mut self) -> Option<Child> {
        None
    }
}
