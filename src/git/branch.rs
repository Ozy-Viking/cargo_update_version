use std::{fmt::Display, str::FromStr};

use clap::builder::OsStr;

#[derive(Debug, PartialEq, Eq, Default, Clone, Hash)]
pub enum Branch {
    #[default]
    Current,
    Named {
        local: String,
    },
}

impl AsRef<str> for Branch {
    fn as_ref(&self) -> &str {
        match self {
            Branch::Current => ".",
            Branch::Named { local } => local,
        }
    }
}

impl Branch {
    /// Returns `true` if the branch is [`Current`].
    ///
    /// [`Current`]: Branch::Current
    #[must_use]
    pub fn is_current(&self) -> bool {
        matches!(self, Self::Current)
    }

    /// Returns `true` if the branch is [`Named`].
    ///
    /// [`Named`]: Branch::Named
    #[must_use]
    pub fn is_named(&self) -> bool {
        matches!(self, Self::Named { .. })
    }

    pub fn as_named(&self) -> Option<&String> {
        if let Self::Named { local } = self {
            Some(local)
        } else {
            None
        }
    }

    pub fn try_into_named(self) -> Result<String, Self> {
        if let Self::Named { local } = self {
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
            Branch::Named { local, .. } => local,
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

        Ok(Self::Named {
            local: String::from(s),
        })
    }
}

impl From<Branch> for clap::builder::OsStr {
    fn from(branch: Branch) -> Self {
        match branch {
            Branch::Current => OsStr::from("."),
            Branch::Named { local } => OsStr::from(local),
        }
    }
}

impl From<String> for Branch {
    fn from(branch: String) -> Self {
        match branch.as_str() {
            "." => Branch::Current,
            _ => Branch::Named { local: branch },
        }
    }
}
