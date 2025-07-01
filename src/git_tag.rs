//! Steps:
//! 1. Check that all not dirty.
//! 2. Bump version and save to file.
//! 3. Add change/hunk.
//! 4. Commit just the hunk with version change.
//! 5. Tag the commit.

use std::{path::Path, process::Command};

use miette::IntoDiagnostic;
use semver::Version;
use tracing::{debug, info, instrument};

pub struct Git;

impl Git {
    #[instrument]
    pub fn is_dirty() -> miette::Result<bool> {
        let mut git_status = Command::new("git");
        git_status.args(["status", "--short"]);
        let out = git_status.output().into_diagnostic()?;

        let out_string = String::from_iter(out.stdout.iter().map(|&i| char::from(i)));
        let count = out_string.lines().count();
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
                    GitFile::parse(out_string)
                        .unwrap_or_default()
                        .iter()
                        .map(|s| "\n  ".to_string() + &s.to_string())
                )
            )
        }
    }

    #[instrument]
    pub fn add_cargo_file(cargo_file: &Path) -> miette::Result<()> {
        let mut git = Command::new("git");
        info!("Staging cargo file");
        git.args(["add", &cargo_file.display().to_string()]);
        git.output().map(|_| ()).into_diagnostic()
    }

    #[instrument]
    pub fn commit(git_message: Option<String>, new_version: &Version) -> miette::Result<()> {
        let mut git = Command::new("git");
        info!("Creating commit");
        git.arg("commit");
        match git_message {
            Some(msg) => {
                git.args(["--message", &msg]);
            }
            None => {
                git.args(["--message", &new_version.to_string()]);
            }
        }

        let out = git.output().into_diagnostic()?;
        let out_string = String::from_iter(out.stdout.iter().map(|&i| char::from(i)));
        dbg!(out_string);
        Ok(())
    }

    pub fn tag(version: &Version) -> miette::Result<()> {
        let mut git = Command::new("git");
        git.args(["tag", &version.to_string()]);
        let _ = git.output().into_diagnostic()?;
        Ok(())
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
