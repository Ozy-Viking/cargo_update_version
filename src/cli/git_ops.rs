#[cfg(feature = "unstable")]
use crate::Branch;
use crate::cli::GIT_HEADER;
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

    #[cfg(feature = "unstable")]
    /// Used to change branch for the execution of the program. Defaults to current branch.
    #[arg(long, default_value = Branch::default(), hide_default_value(true), help_heading = GIT_HEADER)]
    branch: Branch,
}

#[cfg(feature = "unstable")]
impl GitOps {
    pub fn branch(&self) -> Branch {
        self.branch.clone()
    }
}
