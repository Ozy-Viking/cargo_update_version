use clap::ValueEnum;
use clap::builder::OsStr;
use rusty_viking::EnumDisplay;
use std::ffi::OsString;

/// What to suppress from stdout
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, ValueEnum, EnumDisplay)]
#[Lower]
pub enum Suppress {
    #[default]
    None,
    Git,
    Cargo,
    All,
}

impl Suppress {
    /// Returns `true` if the suppress is [`None`].
    ///
    /// [`None`]: Suppress::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if the suppress is [`Git`].
    ///
    /// [`Git`]: Suppress::Git
    #[must_use]
    pub fn is_git(&self) -> bool {
        matches!(self, Self::Git)
    }

    /// Returns `true` if the suppress is [`Git`] or [`All`].
    ///
    /// [`Git`]: Suppress::Git
    /// [`All`]: Suppress::All
    #[must_use]
    pub fn includes_git(&self) -> bool {
        matches!(self, Self::Git | Self::All)
    }

    /// Returns `true` if the suppress is [`Cargo`].
    ///
    /// [`Cargo`]: Suppress::Cargo
    #[must_use]
    pub fn is_cargo(&self) -> bool {
        matches!(self, Self::Cargo)
    }

    /// Returns `true` if the suppress is [`Cargo`] or [`All`].
    ///
    /// [`Cargo`]: Suppress::Cargo
    /// [`All`]: Suppress::All
    #[must_use]
    pub fn includes_cargo(&self) -> bool {
        matches!(self, Self::Cargo | Self::All)
    }

    /// Returns `true` if the suppress is [`All`].
    ///
    /// [`All`]: Suppress::All
    #[must_use]
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }
}

impl From<Suppress> for OsStr {
    fn from(suppress: Suppress) -> Self {
        let string_rep = OsString::from(suppress.to_string());
        Self::from(string_rep)
    }
}
