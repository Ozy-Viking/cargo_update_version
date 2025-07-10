use std::{ffi::OsString, path::PathBuf};

use crate::{GitBuilder, Result};
use cargo_metadata::Metadata;
use clap::builder::OsStr;
use miette::IntoDiagnostic;
use rusty_viking::EnumDisplay;
use semver::Version;
use tracing::{Level, debug, instrument};

use crate::current_span;

static GIT_HEADER: &str = "Git";
pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

#[derive(clap::Parser, Debug)]
#[command(about, long_about=None, version)]
#[command(styles=CLAP_STYLING)]
pub struct Cli {
    /// Action to affect the package version.
    #[arg(default_value_t = Action::default())]
    pub action: Action,

    /// Suppresses stdout out from commands run.
    #[arg(short = 'Q', long)]
    pub supress_stdout: bool,

    /// Runs the `cargo publish`
    #[arg(short, long)]
    pub cargo_publish: bool,

    /// adds 'no_verify' to cargo publish command.
    #[arg(long)]
    pub no_verify: bool,

    #[arg(long, help="Sets the pre-release segment for the new version.", value_parser = semver::Prerelease::new)]
    pub pre: Option<semver::Prerelease>,

    #[arg(long, help = "Sets the build metadata for the new version.")]
    pub build: Option<semver::BuildMetadata>,

    #[arg(short = 'n', long, help = "Allows git tag to occur in a dirty repo.")]
    pub allow_dirty: bool,

    #[command(flatten)]
    pub git_ops: GitOps,

    /// All commands run as if they run in the the directory of the Cargo.toml set.
    #[command(flatten)]
    pub manifest: clap_cargo::Manifest,

    // TODO: Add workplace functionality
    // #[command(flatten)]
    // workspace: clap_cargo::Workspace,
    #[arg(short, long, help = "Bypass version bump checks.")]
    pub force_version: bool,

    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,

    #[arg(short, long, help = "Allows git tag to occur in a dirty repo.")]
    pub dry_run: bool,

    /// New version to set. Ignored if action isn't set.
    #[arg(value_parser = Version::parse)]
    pub set_version: Option<Version>,
}

#[derive(Debug, clap::Args)]
pub struct GitOps {
    #[arg(
        short = 't',
        long,
        help = "Create a git tag.",
        long_help = "Create a git tag. After changing the version, the Cargo.toml and Cargo.lock will be commited and the tag made on this new commit.",
        help_heading = GIT_HEADER
    )]
    pub git_tag: bool,
    #[arg(
        long,
        help = "Push tag to the branch's remote repositries.",
        long_help = "Push tag to the branch's remote repositries. Runs 'git push <remote> tags/<tag>' for each remote.",
        help_heading = GIT_HEADER
    )]
    pub git_push: bool,
    #[arg(short, long, help="Message for git commit. Default to git tag.",
        help_heading = GIT_HEADER
    )]
    pub message: Option<String>,
    #[arg(long = "force-git", help = "Pass force into all git operations.",
        help_heading = GIT_HEADER)]
    pub force: bool,
    #[command(flatten)]
    color: colorchoice_clap::Color,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, clap::ValueEnum, Default, EnumDisplay)]
#[Lower]
pub enum Action {
    #[value(help = "Bump the version 1 patch level.")]
    #[default]
    Patch,
    #[value(help = "Bump the version 1 minor level.")]
    Minor,
    #[value(help = "Bump the version 1 major level.")]
    Major,
    #[value(help = "Set the version using valid semantic versioning.")]
    Set,
    #[value(help = "Print the current version of the package.")]
    Print,
}

impl From<Action> for OsStr {
    fn from(action: Action) -> Self {
        let string_rep = OsString::from(action.to_string());
        Self::from(string_rep)
    }
}

impl Cli {
    pub fn root_dir(&self) -> Result<PathBuf> {
        let root = match self.manifest.manifest_path.clone() {
            Some(p) => p
                .canonicalize()
                .into_diagnostic()?
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| {
                    miette::miette!("Failed to canonicaliaze correctly: {}", &p.display())
                })?,
            None => PathBuf::from("."),
        };
        tracing::info!("Root: {}", &root.display());
        Ok(root)
    }

    #[instrument(skip_all, fields(root_cargo_file), name = "Cli::metadata")]
    pub fn metadata(&self) -> Result<Metadata> {
        let mut cmd = self.manifest.metadata();
        cmd.no_deps();
        let res = cmd.exec().into_diagnostic()?;
        let cargo_file = res.root_package().unwrap().manifest_path.to_string();
        current_span!().record("root_cargo_file", cargo_file);
        tracing::info!("Package metadata found.");
        Ok(res)
    }

    #[instrument(skip_all, fields(self.verbosity), name ="Cli::tracing_level")]
    pub fn tracing_level(&self) -> Option<Level> {
        self.verbosity.tracing_level()
    }

    #[instrument(skip_all, fields(self.action), name ="Cli::action")]
    pub fn action(&self) -> Action {
        let action = self.action;
        tracing::debug!("Action: {}", action);
        action
    }

    #[instrument(skip_all, fields(self.allow_dirty), name ="Cli::allow_dirty")]
    pub fn allow_dirty(&self) -> bool {
        tracing::debug!("allow_dirty");
        self.allow_dirty
    }

    #[instrument(skip_all, fields(self.allow_dirty, count), name ="Cli::try_allow_dirty")]
    pub fn try_allow_dirty(&self) -> Result<()> {
        if self.allow_dirty {
            return Ok(());
        }
        let git = GitBuilder::new().root_directory(self.root_dir()?).build();
        let files: crate::GitFiles = git.dirty_files()?;
        let count = files.len();

        if count != 0 {
            miette::bail!(
                help = "Use '--allow-dirty' to avoid this check.",
                "{} file/s in the working directory contain changes that were not yet committed into git.{}",
                count,
                files
            )
        } else {
            Ok(())
        }
    }

    #[instrument(skip_all, fields(self.dry_run), name ="Cli::dry_run")]
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    #[instrument(skip_all, fields(message), name = "Cli::git_message")]
    pub fn git_message(&self) -> Option<String> {
        let msg = self.git_ops.message.clone();
        current_span!().record("message", &msg);
        tracing::debug!("Fetching the git message if available.");
        msg
    }

    #[instrument(skip_all, fields(self.force_version), name ="Cli::force_version")]
    pub fn force_version(&self) -> bool {
        tracing::debug!("Checking if forcing version.");
        self.force_version
    }

    #[instrument(skip_all, fields(git_tag), name = "Cli::git_tag")]
    pub fn git_tag(&self) -> bool {
        let tag = self.git_ops.git_tag;
        current_span!().record("git_tag", tag);
        debug!("Checking for git tag flag...");
        tag
    }

    #[instrument(skip_all, fields(git_push), name = "Cli::git_push")]
    pub fn git_push(&self) -> bool {
        let push = self.git_ops.git_push;
        current_span!().record("git_push", push);
        debug!("Checking for git push flag...");
        push
    }

    #[instrument(skip_all, fields(cargo_publish), name = "Cli::cargo_publish")]
    pub fn cargo_publish(&self) -> bool {
        let publish = self.cargo_publish;
        current_span!().record("cargo_publish", publish);
        debug!("Checking for cargo publish flag...");
        publish
    }

    pub fn no_verify(&self) -> bool {
        self.no_verify
    }
}
