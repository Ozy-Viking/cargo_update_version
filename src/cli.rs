#![allow(dead_code)]
use clap::{Arg, ArgAction, FromArgMatches, crate_authors, crate_version};

use rusty_viking::IntoDiagnosticWithLocation;

#[derive(Debug, Default)]
pub struct Cli {
    bump_version: BumpVersion,
    dioxus: bool,
    git_tag: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum BumpVersion {
    #[default]
    Patch,
    Minor,
    Major,
    Set(semver::Version),
}

impl BumpVersion {
    /// Returns `true` if the bump version is [`Patch`].
    ///
    /// [`Patch`]: BumpVersion::Patch
    #[must_use]
    pub fn is_patch(&self) -> bool {
        matches!(self, Self::Patch)
    }

    /// Returns `true` if the bump version is [`Minor`].

    ///
    /// [`Minor`]: BumpVersion::Minor
    #[must_use]
    pub fn is_minor(&self) -> bool {
        matches!(self, Self::Minor)
    }

    /// Returns `true` if the bump version is [`Major`].
    ///
    /// [`Major`]: BumpVersion::Major
    #[must_use]
    pub fn is_major(&self) -> bool {
        matches!(self, Self::Major)
    }

    /// Returns `true` if the bump version is [`Set`].
    ///
    /// [`Set`]: BumpVersion::Set
    #[must_use]
    pub fn is_set(&self) -> bool {
        matches!(self, Self::Set(..))
    }

    pub fn as_set(&self) -> Option<&semver::Version> {
        if let Self::Set(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Cli {
    pub fn parse() -> miette::Result<Self> {
        let mut command = clap::Command::new("cargo update_version")
            .version(clap::crate_version!())
            .about("A simple Cargo tool for updating the version in your project.")
            .flatten_help(true)
            .author(crate_authors!())
            .disable_help_subcommand(true)
            .color(clap::ColorChoice::Always)
            .bin_name("cargo");
        let sub_command = clap::Command::new("update_version")
            .author(crate_authors!())
            .about("A simple Cargo tool for updating the version in your project.")
            .version(clap::crate_version!());

        let mut args = Vec::new();
        let patch = Arg::new("patch")
            .short('p')
            .long("patch")
            .action(ArgAction::SetTrue)
            .help("Increment the version by 1 patch level. [default]")
            .help_heading("Version Change (Choose one)")
            .conflicts_with_all(["minor", "major", "set"]);
        args.push(patch);
        let minor = Arg::new("minor")
            .short('m')
            .long("minor")
            .action(ArgAction::SetTrue)
            .help("Increment the version by 1 minor level.")
            .help_heading("Version Change (Choose one)")
            .conflicts_with_all(["patch", "major", "set"]);
        args.push(minor);
        let major = Arg::new("major")
            .short('M')
            .long("major")
            .action(ArgAction::SetTrue)
            .help("Increment the version by 1 major level.")
            .help_heading("Version Change (Choose one)")
            .conflicts_with_all(["patch", "minor", "set"]);
        args.push(major);
        let set_version = Arg::new("set")
            .short('s')
            .long("set")
            .value_name(crate_version!())
            .value_parser(semver::Version::parse)
            .help("Set the version using valid semver.")
            .help_heading("Version Change (Choose one)")
            .conflicts_with_all(["patch", "minor", "major"]);
        args.push(set_version);

        args.push(Arg::new("dioxus").short('d').long("dioxus").action(ArgAction::SetTrue).help("Update all the versions in the dioxus project. Nothing will occur if not in a dioxus project."));
        args.push(
            Arg::new("git_tag")
                .short('t')
                .action(ArgAction::SetTrue)
                .help("Will run git tag as well."),
        );

        command = command.subcommand(sub_command.args(args));

        let matches = command.get_matches();
        Ok(Cli::from_arg_matches(&matches)
            .into_diagnostic_with_help(Some("Error occured with clap.".into()))?)
    }

    pub fn bump_version(&self) -> &BumpVersion {
        &self.bump_version
    }
}

impl FromArgMatches for Cli {
    fn from_arg_matches(mut matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        if let Some(m) = matches.subcommand_matches("update_version") {
            matches = m;
        };
        dbg!(matches);
        let bump_version = if matches.get_flag("patch") {
            BumpVersion::Patch
        } else if matches.get_flag("minor") {
            BumpVersion::Minor
        } else if matches.get_flag("major") {
            BumpVersion::Major
        } else if let Some(v) = matches.get_one::<semver::Version>("set") {
            BumpVersion::Set(v.clone())
        } else {
            BumpVersion::default()
        };
        let dioxus = matches.get_flag("dioxus");
        let git_tag = matches.get_flag("git_tag");
        Ok(Self {
            bump_version,
            dioxus,
            git_tag,
        })
    }

    fn update_from_arg_matches(&mut self, _matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        todo!()
    }
}
