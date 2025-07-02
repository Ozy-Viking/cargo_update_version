#![allow(dead_code)]
use std::{env, error::Error, path::PathBuf};

use clap::{
    Arg, ArgAction, Args, FromArgMatches,
    builder::{Styles, ValueParser, styling::AnsiColor},
    crate_authors, crate_version,
    error::ErrorKind,
};

use clap_verbosity_flag::Verbosity;
use rusty_viking::IntoDiagnosticWithLocation;

type VerbosityLevel = clap_verbosity_flag::WarnLevel;

use crate::error::UvError;

static VERSION_CHANGE_TITLE: &str = "Version Change (Choose one)";

#[derive(Debug, Default)]
pub struct Cli {
    verbosity: Verbosity<VerbosityLevel>,
    allow_dirty: bool,
    bump_version: BumpVersion,
    git_tag: bool,
    git_message: Option<String>,
    manifest_path: Option<PathBuf>,
    print_version: bool,
    force_version: bool,
    git_push: bool,
    publish: bool,
    pub(crate) dry_run: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum BumpVersion {
    #[default]
    Patch,
    Minor,
    Major,
    Set(semver::Version),
}
impl std::fmt::Display for BumpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use BumpVersion as BV;
        match self {
            BV::Patch => write!(f, "Patch"),
            BV::Minor => write!(f, "Minor"),
            BV::Major => write!(f, "Major"),
            BV::Set(version) => write!(f, "Set({})", version),
        }
    }
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
        let mut command = clap::Command::new("cargo-uv")
            .author(crate_authors!())
            .subcommand_required(true)
            .bin_name("cargo")
            .disable_help_subcommand(true)
            // .allow_missing_positional(true)
            .color(clap::ColorChoice::Always)
            .long_about("Intended for use with Cargo.");
        let mut sub_command = clap::Command::new("uv")
            .author(crate_authors!())
            .about("A simple Cargo tool for updating the version in your project.")
            .bin_name("cargo");

        let mut args = Vec::new();
        let patch = Arg::new("patch")
            .short('p')
            .long("patch")
            .action(ArgAction::SetTrue)
            .help("Increment the version by 1 patch level. [default selection]")
            .help_heading(VERSION_CHANGE_TITLE)
            .display_order(0)
            .conflicts_with_all(["minor", "major", "set"]);
        args.push(patch);
        let minor = Arg::new("minor")
            .short('m')
            .long("minor")
            .action(ArgAction::SetTrue)
            .help("Increment the version by 1 minor level.")
            .display_order(1)
            .help_heading(VERSION_CHANGE_TITLE)
            .conflicts_with_all(["patch", "major", "set"]);
        args.push(minor);
        let major = Arg::new("major")
            .short('M')
            .long("major")
            .action(ArgAction::SetTrue)
            .help("Increment the version by 1 major level.")
            .display_order(2)
            .help_heading(VERSION_CHANGE_TITLE)
            .conflicts_with_all(["patch", "minor", "set"]);
        args.push(major);
        let set_version = Arg::new("set")
            .short('s')
            .long("set")
            .value_name(crate_version!())
            .value_parser(semver::Version::parse)
            .display_order(3)
            .help("Set the version using valid semver.")
            .help_heading(VERSION_CHANGE_TITLE)
            .conflicts_with_all(["patch", "minor", "major"]);
        args.push(set_version);
        let manifest_path = Arg::new("manifest-path")
            .help("Path to the Cargo.toml file.")
            .value_name("Path")
            .short('P')
            .value_parser(ValueParser::path_buf())
            .long("manifest-path");
        args.push(manifest_path);

        args.push(
            Arg::new("force_version")
                .short('f')
                .long("force-version")
                .action(ArgAction::SetTrue)
                .help("Force version bump, this will disregard all version checks."),
        );
        args.push(
            Arg::new("git_tag")
                .short('t')
                .long("git-tag")
                .action(ArgAction::SetTrue)
                .help("Will run git tag as well."),
        );
        args.push(
            Arg::new("allow_dirty")
                .short('a')
                .long("allow-dirty")
                .action(ArgAction::SetTrue)
                .help("Allows git tag to occur in a dirty repo."),
        );
        args.push(
            Arg::new("message")
                .short('c')
                .long("message")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .help("Message for git commit. Defaults to new version number."),
        );
        args.push(
            Arg::new("print_version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue)
                .help("Prints the current version of your project then exits."),
        );
        args.push(
            Arg::new("dry_run")
                .short('n')
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Does a dry-run, will create a tag but then deletes it.")
                .long_help("Does a dry-run, will create a tag but then deletes it.\n This is needed for git push.")
        );
        args.push(
            Arg::new("git_push")
                .long("push")
                .action(ArgAction::SetTrue)
                .help("Pushes the tag to all remotes of the current branch not just origin."),
        );
        args.push(
            Arg::new("publish")
                .long("publish")
                .action(ArgAction::SetTrue)
                .help("Runs cargo publish. Allow dirty is required here."),
        );

        sub_command = clap_verbosity_flag::Verbosity::<VerbosityLevel>::augment_args(sub_command);

        let styles = Styles::styled()
            .header(AnsiColor::Yellow.on_default())
            .usage(AnsiColor::Yellow.on_default())
            .literal(AnsiColor::Green.on_default())
            .error(AnsiColor::Red.on_default())
            .valid(AnsiColor::Green.on_default())
            .placeholder(AnsiColor::Cyan.on_default());
        sub_command = sub_command.args(args);

        command = command.subcommand(sub_command.clone()).styles(styles);
        let mut input_args = vec!["cargo".to_string()];
        input_args.extend(
            env::args_os()
                .enumerate()
                .filter(|(i, _)| i != &0)
                .map(|(_, n)| n.into_string().unwrap_or_default())
                .collect::<Vec<String>>(),
        );
        let recommended_given = input_args.join(" ");

        let label_loc = if recommended_given.starts_with("cargo uv ") {
            let offset = "cargo uv ".len();
            let rem = recommended_given.len() - offset;
            (offset, rem)
        } else {
            let offset = "cargo".len();
            let rem = recommended_given.len() - offset;
            (offset, rem)
        };

        // TODO: Remove the uv so it doesn't need the uv `try_get_matches_from`
        let matches = match command.clone().try_get_matches() {
            Ok(m) => m,
            Err(e) => match e.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                | ErrorKind::DisplayVersion => e.exit(),
                kind => {
                    // TODO: Implement conversion to [UvError]

                    if kind == ErrorKind::ValueValidation {
                        if let Some(inner) = e.source() {
                            if let Some(semver_error) = inner.downcast_ref::<semver::Error>() {
                                let _ = super::error::UvError::Semver {
                                    msg: semver_error.to_string(),
                                    source_code: recommended_given,
                                    help: "Minimum valid semver is major.minor.patch(0.2.1)."
                                        .into(),
                                };
                            };
                        }
                    } else {
                        dbg!(&e);
                        Err(UvError::Clap {
                            label: Some(label_loc),
                            label_msg: e.kind().as_str().unwrap_or("Error here"),
                            kind: e.kind(),
                            source_code: recommended_given,
                            msg: e.render().ansi().to_string(),
                            help: sub_command.render_usage().ansi().to_string(),
                        })?;
                    }
                    e.exit();
                }
            },
        };
        // let matches = command.get_matches();
        Cli::from_arg_matches(&matches)
            .into_diagnostic_with_help(Some("Error occured with clap.".into()))
    }

    pub fn bump_version(&self) -> &BumpVersion {
        &self.bump_version
    }

    pub fn manifest_path(&self) -> Option<&PathBuf> {
        self.manifest_path.as_ref()
    }

    pub fn verbosity(&self) -> Verbosity<VerbosityLevel> {
        self.verbosity
    }

    pub fn git_message(&self) -> Option<String> {
        self.git_message.clone()
    }

    pub fn allow_dirty(&self) -> bool {
        self.allow_dirty
    }

    pub fn git_tag(&self) -> bool {
        self.git_tag
    }

    pub fn print_version(&self) -> bool {
        self.print_version
    }

    pub fn force(&self) -> bool {
        self.force_version
    }

    pub fn git_push(&self) -> bool {
        self.git_push
    }

    pub fn publish(&self) -> bool {
        self.publish
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }
}

impl FromArgMatches for Cli {
    fn from_arg_matches(mut matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        if let Some(m) = matches.subcommand_matches("uv") {
            matches = m;
        };
        // dbg!(matches);
        let mut verbosity = Verbosity::<VerbosityLevel>::default();
        verbosity.update_from_arg_matches(matches)?;
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
        let git_tag = matches.get_flag("git_tag");
        let allow_dirty = matches.get_flag("allow_dirty");
        let print_version = matches.get_flag("print_version");
        let force_version = matches.get_flag("force_version");
        let git_push = matches.get_flag("git_push");
        let dry_run = matches.get_flag("dry_run");
        let publish = matches.get_flag("publish");
        let manifest_path = matches.get_one::<PathBuf>("manifest-path").cloned();
        let mut git_message = matches.get_one::<String>("message").cloned();
        if let Some(git_msg) = git_message.clone() {
            if git_msg.trim().is_empty() {
                git_message = None
            }
        };

        Ok(Self {
            verbosity,
            bump_version,
            git_tag,
            allow_dirty,
            git_message,
            manifest_path,
            print_version,
            force_version,
            git_push,
            publish,
            dry_run,
        })
    }

    fn update_from_arg_matches(&mut self, _matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        unimplemented!()
    }
}
