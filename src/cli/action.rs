use std::ffi::OsString;

use clap::builder::OsStr;
use rusty_viking::EnumDisplay;

#[derive(Debug, PartialEq, Eq, Clone, Copy, clap::ValueEnum, Default, EnumDisplay, Hash)]
#[Lower]
pub enum Action {
    #[value(help = "Bump the version 1 prerelease level.", hide(true))]
    // TODO: Remove when implemented.
    Pre,
    #[value(help = "Bump the version 1 patch level.")]
    Patch,
    #[value(help = "Bump the version 1 minor level.")]
    Minor,
    #[value(help = "Bump the version 1 major level.")]
    Major,
    #[value(help = "Set the version using valid semantic versioning.")]
    Set,
    #[value(help = "Print the current version of the package.")]
    #[default]
    Print,
    /// Display the layout of the members in the workspace.
    Tree,
}

impl From<Action> for OsStr {
    fn from(action: Action) -> Self {
        let string_rep = OsString::from(action.to_string());
        Self::from(string_rep)
    }
}
