use std::{fmt::Display, str::FromStr};

use clap::builder::OsStr;

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

    /// Used to change branch for the execution of the program. Defaults to current branch.
    #[arg(long, default_value = Branch::default(), hide_default_value(true), help_heading = GIT_HEADER)]
    branch: Branch,
}
impl GitOps {
    pub fn branch(&self) -> Branch {
        self.branch.clone()
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub enum Branch {
    #[default]
    Current,
    Other {
        local: String,
    },
}

impl Branch {
    /// Returns `true` if the branch is [`Current`].
    ///
    /// [`Current`]: Branch::Current
    #[must_use]
    pub fn is_current(&self) -> bool {
        matches!(self, Self::Current)
    }

    /// Returns `true` if the branch is [`Other`].
    ///
    /// [`Other`]: Branch::Other
    #[must_use]
    pub fn is_other(&self) -> bool {
        matches!(self, Self::Other { .. })
    }

    pub fn as_other(&self) -> Option<&String> {
        if let Self::Other { local } = self {
            Some(local)
        } else {
            None
        }
    }

    pub fn try_into_other(self) -> Result<String, Self> {
        if let Self::Other { local } = self {
            Ok(local)
        } else {
            Err(self)
        }
    }
}

impl Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Branch::Current => ".",
            Branch::Other { local, .. } => &local,
        };

        write!(f, "{text}")
    }
}

impl FromStr for Branch {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() || s == "." {
            return Ok(Branch::Current);
        }

        Ok(Self::Other {
            local: String::from(s),
        })
    }
}

impl From<Branch> for clap::builder::OsStr {
    fn from(branch: Branch) -> Self {
        match branch {
            Branch::Current => OsStr::from("."),
            Branch::Other { local } => OsStr::from(local),
        }
    }
}
