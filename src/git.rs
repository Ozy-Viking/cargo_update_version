//! Steps:
//! 1. Check that all not dirty.
//! 2. Bump version and save to file.
//! 3. Add change/hunk.
//! 4. Commit just the hunk with version change.
//! 5. Tag the commit.

pub(crate) mod git_file;

use std::{
    path::Path,
    process::{Child, Command, Output, Stdio},
};

use miette::{Context, IntoDiagnostic, bail};
use semver::Version;
use tracing::{debug, info, instrument, warn};

use crate::{cli::Cli, git::git_file::GitFiles};

// TODO: Use the directory of the cargo file maybe /workspace.
// TODO: Push branch as well as flag.
pub struct Git;

impl Git {
    /// Generates a list of dirty files.
    #[instrument]
    pub fn dirty_files() -> miette::Result<GitFiles> {
        let mut git_status = Git::command();
        git_status.args(["status", "--short"]);
        let stdout = git_status.output().into_diagnostic()?.stdout();
        if stdout.lines().count() == 0 {
            return Ok(GitFiles::new());
        };
        match GitFiles::parse(stdout) {
            Some(files) => Ok(files),
            None => Ok(GitFiles::new()),
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
        git.args(["commit"]);

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
        // dbg!(&git);

        // TODO: Output of commited files.
        let _stdout = git.output().into_diagnostic()?;
        Git::dirty_files().context("After Commit")?;
        Ok(())
    }

    #[instrument]
    pub fn tag(_cli_args: &Cli, version: &Version, args: Option<Vec<&str>>) -> miette::Result<()> {
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
    pub fn generate_tag(version: &Version) -> String {
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
    #[instrument]
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

#[allow(dead_code)]
pub trait OutputExt {
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
