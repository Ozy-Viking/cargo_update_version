//! Steps:
//! 0. Check that all not dirty.
//! 1. Bump version and save to file.
//! 2. Add change/hunk.
//! 3. Commit just the hunk with version change.
//! 4. Tag the commit.

use std::process::Command;

use miette::IntoDiagnostic;
use tracing::instrument;

pub struct Git;

impl Git {
    #[instrument]
    pub fn is_dirty() -> miette::Result<bool> {
        let mut git_status = Command::new("git");
        git_status.args(["status", "--short"]);
        let out = git_status.output().into_diagnostic()?;

        let out_string = String::from_iter(out.stdout.iter().map(|&i| char::from(i)));
        dbg!(out_string.lines().count());
        Ok(true)
    }
}
