use cargo_metadata::Metadata;
use miette::{IntoDiagnostic, Result};
use rusty_viking::EnumDisplay;
use tracing::{Level, instrument};

use crate::current_span;

static GIT_HEADER: &str = "Git";
static OPTIONS_HEADER: &str = "Options";
pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

#[derive(clap::Parser, Debug)]
#[command(name = "uv")]
#[command(about, long_about=None, version)]
#[command(styles=CLAP_STYLING)]
pub struct Cli {
    #[arg(default_value_t = Action::default())]
    pub action: Action,
    #[arg(short, long, help = "Run Cargo Publish.")]
    pub cargo_publish: bool,

    #[arg(long, help="Sets the pre-release segment for the new version.", value_parser = semver::Prerelease::new)]
    pub pre: Option<semver::Prerelease>,
    #[arg(long, help = "Sets the build metadata for the new version.")]
    pub build: Option<semver::BuildMetadata>,
    #[arg(short = 'n', long, help = "Allows git tag to occur in a dirty repo.")]
    pub allow_dirty: bool,

    #[command(flatten)]
    pub git_ops: GitOps,

    #[command(flatten)]
    manifest: clap_cargo::Manifest,

    // TODO: Add workplace
    // #[command(flatten)]
    // workspace: clap_cargo::Workspace,
    #[arg(short, long, help = "Bypass version bump checks.")]
    pub force_version: bool,
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,
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

impl Cli {
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

    #[instrument]
    pub fn tracing_level(&self) -> Option<Level> {
        self.verbosity.tracing_level()
    }

    #[instrument(skip_all, fields(action))]
    pub fn action(&self) -> Action {
        let action = self.action;
        current_span!().record("action", action.to_string());
        tracing::debug!("Action: {}", action);
        action
    }
}
