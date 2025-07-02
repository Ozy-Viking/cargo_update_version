//! Steps:
//! 1. Check that all not dirty.
//! 2. Bump version and save to file.
//! 3. Add change/hunk.
//! 4. Commit just the hunk with version change.
//! 5. Tag the commit.

use std::{
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
};

use miette::{IntoDiagnostic, bail, miette};
use semver::Version;
use tracing::{debug, info, instrument, warn};

use crate::cli::Cli;

pub struct Git;

impl Git {
    #[instrument]
    pub fn is_dirty() -> miette::Result<bool> {
        let mut git_status = Git::command();
        git_status.args(["status", "--short"]);
        let stdout = git_status.output().into_diagnostic()?.stdout();

        let count = stdout.lines().count();
        if count == 0 {
            info!("Git is clean");
            Ok(true)
        } else {
            debug!("Git stage is dirty: {} files", count);
            miette::bail!(
                help = "Use '--allow-dirty' to avoid this check.",
                "{} file/s in the working directory contain changes that were not yet committed into git.{}",
                count,
                String::from_iter(
                    GitFile::parse(stdout)
                        .unwrap_or_default()
                        .iter()
                        .map(|s| "\n  ".to_string() + &s.to_string())
                )
            )
        }
    }

    #[instrument]
    pub fn add_cargo_files(cli_args: &Cli, cargo_file: &Path) -> miette::Result<()> {
        let mut git = Git::command();
        let cargo_lock = cargo_file
            .to_path_buf()
            .parent()
            .unwrap()
            .join("Cargo.lock");
        info!("Staging cargo file");
        git.args([
            "add",
            &cargo_file.display().to_string(),
            &cargo_lock.display().to_string(),
        ]);
        git.output().map(|_| ()).into_diagnostic()
    }

    #[instrument]
    pub fn commit(cli_args: &Cli, new_version: &Version) -> miette::Result<()> {
        let mut git = Git::command();
        info!("Creating commit");
        git.args(["commit", "--short"]);

        if cli_args.dry_run() {
            git.arg("--dry-run");
        }
        match cli_args.git_message() {
            Some(msg) => {
                git.args(["--message", &msg]);
            }
            None => {
                git.args(["--message", &new_version.to_string()]);
            }
        }

        // TODO: Output of commited files.
        let _stdout = dbg!(git.output().into_diagnostic()?);
        Ok(())
    }

    pub fn tag(cli_args: &Cli, version: &Version, args: Option<Vec<&str>>) -> miette::Result<()> {
        // if cli_args.dry_run() {
        //     info!(
        //         dry_run = true,
        //         "Would of taged: {}",
        //         Git::generate_tag(version)
        //     );
        //     return Ok(());
        // }
        let mut git = Git::command();
        git.arg("tag");
        if let Some(a) = args {
            git.args(a);
        }
        git.args([&Git::generate_tag(version)]);
        let _output = git.output().into_diagnostic()?;

        Ok(())
    }

    #[instrument]
    pub fn generate_tag(ref version: &Version) -> String {
        let tag = version.to_string();
        debug! {"Tag: {tag}", };
        tag
    }

    /// Pushed just the tag to the remotes
    #[instrument(skip_all)]
    pub fn push(cli_args: &Cli, version: &Version) -> miette::Result<Vec<Child>> {
        let tag_string = String::from("tags/") + &Git::generate_tag(version);
        let join = Git::remotes()?
            .iter()
            .map(|remote| {
                info!("Pushing to remote: {remote}");
                let mut git_push = Git::command();
                git_push.arg("push");
                if cli_args.dry_run() {
                    git_push.arg("--dry-run");
                }
                git_push.args([remote.as_str(), &tag_string, "--porcelain"]);
                git_push.stdout(Stdio::null());
                git_push.stderr(Stdio::null());
                // let _ = dbg!(git_push.get_args());
                git_push.spawn().into_diagnostic()
            })
            .collect::<Vec<_>>();
        let mut ret = vec![];

        for jh in join {
            ret.push(jh?);
        }

        Ok(ret)
    }

    /// Returns a list of remotes for the current branch.
    ///
    /// Returns an error if the list is empty
    pub fn remotes() -> miette::Result<Vec<String>> {
        let mut git = Git::command();
        git.args(["remote"]);
        let remotes: Vec<String> = git
            .output()
            .into_diagnostic()?
            .stdout()
            .lines()
            .map(String::from)
            .collect();

        let mut branch_remotes = Vec::new();

        for line in Git::branch(vec!["--remotes"])?.lines() {
            let valid_remote = match line.split_once('/') {
                Some((remote, _branch)) => remote.trim().to_string(),
                None => {
                    warn!("Ensure you only run command on a branch with a remote.");
                    bail!("Failed to find remote for current branch.")
                }
            };
            assert!(remotes.contains(&valid_remote));

            branch_remotes.push(valid_remote);
        }
        info!("Remotes: {:?}", branch_remotes);

        assert!(!branch_remotes.is_empty());
        assert!(remotes.len() >= branch_remotes.len());
        if branch_remotes.is_empty() {
            warn!("Ensure you only run command on a branch with a remote.");
            bail!("Failed to find remote for current branch.")
        }
        Ok(branch_remotes)
    }

    #[instrument]
    pub fn branch(args: Vec<&str>) -> miette::Result<String> {
        let mut git = Git::command();
        git.arg("branch");
        args.iter().for_each(|&arg| {
            git.arg(arg);
        });
        git.output().map(|output| output.stdout()).into_diagnostic()
    }

    /// Base git command
    fn command() -> Command {
        Command::new("git")
    }
}

trait OutputExt {
    fn stderr(&self) -> String;
    fn stdout(&self) -> String;
}

impl OutputExt for Output {
    fn stderr(&self) -> String {
        String::from_iter(self.stderr.iter().map(|&c| char::from(c)))
    }

    fn stdout(&self) -> String {
        String::from_iter(self.stdout.iter().map(|&c| char::from(c)))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct GitFile {
    pub mode: String,
    pub path: String,
}

impl std::fmt::Display for GitFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl GitFile {
    #[instrument]
    pub fn parse(input: String) -> Option<Vec<GitFile>> {
        let lines = input.lines();
        let mut ret = Vec::new();
        for line in lines {
            let (mode, path) = line.trim().split_once(" ")?;
            ret.push(GitFile {
                mode: mode.to_string(),
                path: path.to_string(),
            });
        }
        if ret.is_empty() { None } else { Some(ret) }
    }
}
