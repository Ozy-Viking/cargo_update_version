use std::{fmt::Display, path::PathBuf, process::Child};

use semver::{BuildMetadata, Prerelease, Version};
use tracing::instrument;

use crate::{
    Action, Branch, Bumpable, Cargo, Cli, Git, Package, PackageName, Packages, ReadToml, Result,
    Stash,
};

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
    #[cfg(feature = "unstable")]
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
    #[cfg(feature = "unstable")]
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
            #[cfg(feature = "unstable")]
            Task::GitSwitchBranch { to, .. } => &format!("Change branch: {}", to),
            Task::GitAdd(paths) => &format!("Git Add: {:#?}", paths),
            #[cfg(feature = "unstable")]
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
#[allow(rustdoc::invalid_html_tags)]
/// As_<Enum type> implementations
impl Task {
    pub fn as_git_push(&self) -> Option<&Task> {
        match self {
            Task::GitPush { .. } => Some(self),
            _ => None,
        }
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

    /// Returns `true` if the task is [`DisplayVersion`].
    ///
    /// [`DisplayVersion`]: Task::DisplayVersion
    #[must_use]
    pub fn is_display_version(&self) -> bool {
        matches!(self, Self::DisplayVersion(..))
    }

    /// Returns `true` if the task is [`GitPush`].
    ///
    /// [`GitPush`]: Task::GitPush
    #[must_use]
    pub fn is_git_push(&self) -> bool {
        matches!(self, Self::GitPush { .. })
    }

    #[cfg(feature = "unstable")]
    /// Returns `true` if the task is [`GitSwitchBranch`].
    ///
    /// [`GitSwitchBranch`]: Task::GitSwitchBranch
    #[must_use]
    pub fn is_git_switch_branch(&self) -> bool {
        matches!(self, Self::GitSwitchBranch { .. })
    }

    pub fn is_run_after_completed(&self) -> bool {
        self.is_delete_git_tag()
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
    pub fn run(
        &self,
        cli_args: &Cli,
        packages: &mut Packages,
        git: &Git<PathBuf>,
        cargo: &Cargo,
    ) -> Result<Option<Child>> {
        tracing::debug!("Starting task: {}", self);
        let dry_run = cli_args.dry_run();
        let no_verify = cli_args.no_verify();
        let allow_dirty = cli_args.allow_dirty();
        let root_version = packages.root_version()?;
        let suppress = cli_args.suppress();
        let ret: Result<Option<Child>> = match self {
            Task::GitPush { remote, tag, .. } => {
                git.push(tag, suppress, dry_run, remote).map(|c| Some(c))
            }
            Task::CargoPublish => cargo
                .publish(suppress, dry_run, no_verify, allow_dirty)
                .map(|c| Some(c)),
            Task::DisplayVersion(package_name) => {
                let package = packages
                    .get_package(package_name)
                    .ok_or(miette::miette!("No package with name {}", package_name))?;
                println!("{} {}", package_name, package.version());
                Ok(None)
            }
            Task::WorkspaceTree => {
                println!("{}", packages.display_tree());
                Ok(None)
            }
            Task::Set {
                package_name,
                new_version,
            }
            | Task::Bump {
                package_name,
                new_version,
                ..
            } => packages
                .set_package_version(package_name, new_version.clone())
                .map(|_| None),
            Task::SetWorkspace { new_version } | Task::BumpWorkspace { new_version, .. } => {
                packages
                    .set_workspace_package_version(new_version.clone())
                    .map(|_| None)
            }
            Task::DeleteGitTag(version) => git
                .tag(version, suppress, Some(vec!["--delete"]))
                .map(|_| None),
            #[cfg(feature = "unstable")]
            Task::GitSwitchBranch { to, .. } => git.checkout(to, suppress).map(|_| None),
            Task::WriteCargoToml(package_name) => {
                packages.write_cargo_file(package_name).map(|_| None)
            }

            #[cfg(feature = "unstable")]
            Task::GitStash { .. } => todo!(),
            Task::GitAdd(files) => git.add_files(files).map(|_| None),
            Task::GitCommit => git
                .commit(
                    &cli_args.git_message().unwrap_or(root_version.to_string()),
                    suppress,
                    dry_run,
                )
                .map(|_| None),
            Task::GitTag(version) => git.tag(version, suppress, None).map(|_| None),
            Task::CargoGenerateLock => cargo.generate_lockfile().map(|_| None),
        };
        tracing::trace!("Finishing task: {} with status Ok:{}", self, ret.is_ok());
        ret
    }
}
