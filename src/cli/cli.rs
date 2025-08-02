use std::{ops::Deref, path::PathBuf};

use crate::{
    Action, Branch, GitBuilder, Result,
    cli::{CARGO_HEADER, GitOps, Manifest, Suppress, Workspace},
};
use cargo_metadata::Metadata;
use miette::IntoDiagnostic;
use semver::Version;
use tracing::{Level, debug, instrument};

use crate::current_span;
// use clap::ValueHint;

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

    #[arg(long, help="Sets the pre-release segment for the new version.", value_parser = semver::Prerelease::new)]
    pub pre: Option<semver::Prerelease>,

    #[arg(long, help = "Sets the build metadata for the new version.")]
    pub build: Option<semver::BuildMetadata>,

    /// Runs the `cargo publish`
    #[arg(short, long, help_heading = CARGO_HEADER)]
    pub cargo_publish: bool,

    /// What to suppress from stdout
    #[arg(short = 'Q', long, default_value = Suppress::default())]
    pub suppress: Suppress,

    /// adds 'no_verify' to cargo publish command.
    #[arg(long, help_heading = CARGO_HEADER)]
    pub no_verify: bool,

    #[arg(short = 'n', long, help = "Allows program to work in a dirty repo.")]
    pub allow_dirty: bool,

    #[command(flatten)]
    pub git_ops: GitOps,

    /// All commands run as if they run in the the directory of the Cargo.toml set.
    #[command(flatten)]
    pub manifest: Manifest,

    #[command(flatten)]
    pub workspace: Workspace,

    #[arg(short, long, help = "Bypass version bump checks.")]
    pub force_version: bool,

    #[arg(short, long, help = "Allows git tag to occur in a dirty repo.")]
    pub dry_run: bool,

    #[command(flatten)]
    pub color: colorchoice_clap::Color,

    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,

    /// New version to set. Ignored if action isn't set.
    #[arg(value_parser = Version::parse)]
    pub set_version: Option<Version>,

    #[arg(skip)]
    metadata: Option<Metadata>,

    /// Display the tasks that will be run.
    display_tasks: bool,
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

    #[instrument(skip_all, fields(root_cargo_file), name = "Cli::get_metadata")]
    pub fn get_metadata<'m>(&'m mut self) -> Result<&'m Metadata> {
        if let Some(ref m) = self.metadata {
            Ok(m)
        } else {
            self.refresh_metadata()?;
            let cargo_file = self
                .metadata
                .as_ref()
                .unwrap()
                .workspace_root
                .join("Cargo.toml")
                .to_string();
            current_span!().record("root_cargo_file", cargo_file);
            tracing::info!("Package metadata found.");
            self.metadata
                .as_ref()
                .ok_or_else(|| miette::miette!("Failed to get metadata somehow..."))
        }
    }

    #[instrument(skip_all, fields(root_cargo_file), name = "Cli::refresh_metadata")]
    pub fn refresh_metadata(&mut self) -> Result<()> {
        let mut cmd = self.manifest.metadata();
        cmd.no_deps(); // Confirmed does have an impact on performance.
        self.metadata = Some(cmd.exec().into_diagnostic()?);
        Ok(())
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

    pub fn git_branch(&self) -> Branch {
        self.git_ops.branch()
    }

    pub fn is_current_branch(&self) -> bool {
        self.git_branch().is_current()
    }

    pub fn display_tasks(&self) -> bool {
        self.display_tasks
    }

    // /// Partition workspace members into those selected and those excluded.
    // ///
    // /// Notes:
    // /// - Requires the features `cargo_metadata`.
    // /// - Requires not calling `MetadataCommand::no_deps`
    // pub fn partition_packages<'p>(
    //     &'p mut self,
    // ) -> Result<(
    //     Vec<&'p cargo_metadata::Package>,
    //     Vec<&'p cargo_metadata::Package>,
    // )> {
    //     self.refresh_metadata()?;
    //     match self.metadata() {
    //         Some(meta) => Ok(self.workspace.partition_packages(meta)),
    //         None => bail!("Metadata not fetched."),
    //     }
    // }
}

impl Deref for Cli {
    type Target = Workspace;

    fn deref(&self) -> &Workspace {
        &self.workspace
    }
}

impl Cli {
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }
}
