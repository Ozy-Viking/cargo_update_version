use std::{fmt::Display, path::PathBuf, process::Child};

use semver::{BuildMetadata, Prerelease, Version};
use tracing::instrument;

use crate::{Action, Branch, Bumpable, Package, PackageName, Packages, ReadToml, Result, Stash};

#[derive(Hash, PartialEq, Debug, Eq, Clone)]
pub enum Task {
    // Display
    DisplayVersion(PackageName),
    WorkspaceTree,

    // Version adjustment
    Set {
        package_name: PackageName,
        new_version: Version,
    },
    SetWorkspace {
        new_version: Version,
    },
    Bump {
        package_name: PackageName,
        bump: Action,
        new_version: Version,
    },
    BumpWorkspace {
        bump: Action,
        new_version: Version,
    },

    // Git
    GitStash {
        branch: Branch,
        stash: Stash,
    },
    GitAdd(Vec<PathBuf>),
    GitCommit,
    GitPush {
        remote: String,

        #[cfg(feature = "unstable")]
        branch: Branch,
        tag: String,
    },
    GitSwitchBranch {
        to: Branch,
        from: Branch,
    },
    GitTag(Version),
    DeleteGitTag(Version),

    // Cargo
    WriteCargoToml(PackageName),
    CargoPublish,
    CargoGenerateLock,
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Task::DisplayVersion(package) => &format!("Print Version: {}", package),
            Task::WorkspaceTree => "Display Workspace Tree",
            Task::Bump {
                package_name: package,
                bump,
                new_version,
            } => &format!("Bump {bump}: {} -> {new_version}", package),
            Task::BumpWorkspace { bump, .. } => &format!("Bump Workspace Package: {}", bump),
            Task::Set {
                new_version,
                package_name: package,
            } => &format!("Set {}: {}", package, new_version),
            Task::SetWorkspace {
                new_version: version,
            } => &format!("Set Workspace: {}", version.to_string()),
            Task::CargoPublish => "Cargo Publish",
            Task::WriteCargoToml(package) => &format!("Write Cargo.toml for: {}", package),
            Task::GitSwitchBranch { to, .. } => &format!("Change branch: {}", to),
            Task::GitAdd(paths) => &format!("Git Add: {:#?}", paths),
            Task::GitStash {
                branch,
                stash: state,
            } => &format!("Git Stash: {state:?} files on {}", branch),
            #[cfg(feature = "unstable")]
            Task::GitPush {
                remote,
                branch,
                tag,
            } => &format!("Git Push: {tag} to {remote} on {branch}"),

            #[cfg(not(feature = "unstable"))]
            Task::GitPush { remote, tag } => &format!("Git Push: {tag} to {remote}"),
            Task::GitCommit => "Git Commit",
            Task::GitTag(version) => &format!("Git Tag: {}", version),
            Task::DeleteGitTag(version) => &format!("Delete Git Tag: {}", version.to_string()),
            Task::CargoGenerateLock => "Cargo Generate Lockfile",
        };
        write!(f, "{}", text)
    }
}

impl Task {
    pub fn is_version_change(&self) -> bool {
        match self {
            Task::Set { .. }
            | Task::Bump { .. }
            | Task::BumpWorkspace { .. }
            | Task::SetWorkspace { .. } => true,
            _ => false,
        }
    }

    /// Returns `true` if the task is [`DeleteGitTag`].
    ///
    /// [`DeleteGitTag`]: Task::DeleteGitTag
    #[must_use]
    pub fn is_delete_git_tag(&self) -> bool {
        matches!(self, Self::DeleteGitTag(..))
    }

    /// Returns `true` if the task is [`ChangeBranch`].
    ///
    /// [`ChangeBranch`]: Task::ChangeBranch
    #[must_use]
    pub fn is_change_branch(&self) -> bool {
        matches!(self, Self::GitSwitchBranch { .. })
    }

    /// Returns `true` if the task is [`Print`].
    ///
    /// [`Print`]: Task::Print
    #[must_use]
    pub fn is_print(&self) -> bool {
        matches!(self, Self::DisplayVersion(..))
    }
}

/// TODO: Make a reference.
impl<'a> Task {
    pub fn from_action(
        action: Action,
        package: &'a Package<ReadToml>,
        set_version: Option<Version>,
        pre_release: Option<&Prerelease>,
        build: Option<&BuildMetadata>,
        force_version: bool,
    ) -> Result<Task> {
        match action {
            Action::Pre | Action::Patch | Action::Minor | Action::Major => {
                let mut new_version = package.version_owned();
                new_version.bump(action, pre_release, build, force_version)?;
                Ok(Task::Bump {
                    package_name: package.name().clone(),
                    bump: action,
                    new_version,
                })
            }
            Action::Set => Ok(Task::Set {
                new_version: set_version.ok_or(miette::miette!(
                    "Expected a version for Task::from_action when the action is `Set`"
                ))?,
                package_name: package.name().clone(),
            }),
            Action::Tree => Ok(Task::WorkspaceTree),
            Action::Print => Ok(Task::DisplayVersion(package.name().clone())),
        }
    }
}

impl Task {
    #[track_caller]
    #[instrument(name = "Task::run()")]
    /// Run the core function for the task.
    pub fn run(&mut self, packages: Packages) -> Result<Option<Child>> {
        tracing::debug!("Starting task: {}", self);
        let ret = match self {
            Task::GitPush { .. } => todo!(),
            Task::CargoPublish => todo!(),
            Task::DisplayVersion(package_name) => {
                let package = packages
                    .get_package(package_name)
                    .ok_or(miette::miette!("No package with name {}", package_name))?;
                println!("{}: {}", package_name, package.version());
                Ok(None)
            }
            Task::WorkspaceTree => {
                println!("{}", packages.display_tree());
                Ok(None)
            }
            Task::Set { .. } => todo!(),
            Task::Bump { .. } => todo!(),
            Task::BumpWorkspace { .. } => todo!(),
            Task::SetWorkspace { .. } => todo!(),
            Task::DeleteGitTag(_) => todo!(),
            Task::GitSwitchBranch { .. } => todo!(),
            Task::WriteCargoToml(_) => todo!(),
            Task::GitStash { .. } => todo!(),
            Task::GitAdd(_) => todo!(),
            Task::GitCommit => todo!(),
            Task::GitTag(_) => todo!(),
            Task::CargoGenerateLock => todo!(),
        };
        tracing::trace!("Finishing task: {} with status Ok:{}", self, ret.is_ok());
        ret
    }
}
