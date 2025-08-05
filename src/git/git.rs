use std::{
    fmt::{Debug, Display},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    str::FromStr,
};

use indexmap::IndexSet;
use miette::{Context, bail};
use semver::Version;
use tracing::{debug, info, instrument, warn};

use crate::{
    Branch, Process, ProcessOutput, Result, Task,
    cli::{Cli, Suppress},
    current_span,
    git::git_file::GitFiles,
    process::OutputExt,
};

/// Used to indicate if the Root Dir is Set and can be used.
#[derive(Debug)]
pub struct NoRootDirSet;

#[derive(Debug, Default)]
pub struct GitBuilder<T: Debug> {
    root_directory: T,
}
impl GitBuilder<NoRootDirSet> {
    pub fn new() -> Self {
        Self {
            root_directory: NoRootDirSet,
        }
    }
}
impl std::fmt::Display for GitBuilder<NoRootDirSet> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No root directory set for GitBuilder.")
    }
}
impl std::error::Error for GitBuilder<NoRootDirSet> {}

impl<T: Debug> GitBuilder<T> {
    /// Manually set the root directory of the project i.e. where .git lives.
    pub fn root_directory(self, path: PathBuf) -> GitBuilder<PathBuf> {
        GitBuilder {
            root_directory: path,
        }
    }

    /// Use git to locate the root directory using:
    ///
    /// ```shell
    /// git rev-parse --show-toplevel
    /// ```
    #[instrument]
    pub fn find_root_directory(self) -> Result<GitBuilder<PathBuf>, Self> {
        let mut git = Git::<NoRootDirSet>::command(true);
        git.arg("rev-parse").arg("--show-toplevel");
        let out = match git.output() {
            Ok(o) => o.stdout(),
            Err(_) => {
                tracing::error!("Could not find git root dir.");
                return Err(self);
            }
        };
        let path = PathBuf::from_str(&out).map_err(|_| self)?;
        Ok(GitBuilder {
            root_directory: path,
        })
    }
}

impl GitBuilder<PathBuf> {
    pub fn build(self) -> Git<PathBuf> {
        Git {
            root_directory: self.root_directory,
        }
    }
}

#[derive(Debug)]
pub struct Git<T: Debug> {
    root_directory: T,
}

impl Git<NoRootDirSet> {
    #[instrument(name = "Git::command")]
    /// Base git command run in cwd.
    fn command(quiet: bool) -> Command {
        let mut cmd = Command::new("git");
        if !quiet {
            cmd.stdout(Stdio::inherit());
        }
        // cmd.stderr(Stdio::piped());
        cmd
    }
}

impl Git<PathBuf> {
    /// Base git command run in set root path.
    #[instrument(name = "Git::command", skip_all)]
    fn command(&self, quiet: bool) -> Command {
        let mut cmd = Command::new("git");
        // cmd.current_dir(&self.root_directory);
        cmd.arg("-C")
            .arg(self.root_directory.clone().into_os_string());
        tracing::trace!("Command: {:#?}", &cmd);
        if !quiet {
            cmd.stdout(Stdio::inherit());
        }
        cmd
    }

    pub fn root_directory(&self) -> &Path {
        &self.root_directory
    }

    #[instrument(skip_all)]
    /// Adds all cargo files (Cargo.toml, Cargo.lock) in whole project to git.
    ///
    /// Equivilent to: `git add Cargo.toml Cargo.lock`
    ///
    /// TODO: Confirm if file is in git ignore it doesn't add them.
    /// BUG: #28 Git add fetal if doesn't match path spec. Change to generate adds of known files.
    /// add 'Cargo.lock'
    /// add 'Cargo.toml'
    /// add 'pack1/Cargo.toml'
    /// add 'pack2/Cargo.toml'
    pub fn add_cargo_files(&self) -> miette::Result<()> {
        let mut git = self.command(false);
        let cargo_toml = "Cargo.toml";
        let all_cargo_toml = "./**/Cargo.toml";
        let cargo_lock = "Cargo.lock";

        info!("Staging cargo files: {}, {}", cargo_toml, cargo_lock);
        git.args(["add", "-v", cargo_toml, cargo_lock, all_cargo_toml]);
        Process::Output.run(git).map(|_| ())
    }
}

impl Git<PathBuf> {
    /// Generates a [GitFiles] of dirty files. Only errors if the command errors.
    #[instrument(skip_all)]
    pub fn dirty_files(&self) -> miette::Result<GitFiles> {
        let mut git = self.command(true);
        git.args(["status", "--short"]);
        let stdout = match Process::Output.run(git)? {
            ProcessOutput::Output(output) => {
                if output.status.success() {
                    output.stdout()
                } else {
                    bail!("'git status --short' failed")
                }
            }
            _ => unreachable!(),
        };
        if stdout.lines().count() == 0 {
            return Ok(GitFiles::new());
        };
        match GitFiles::parse(stdout) {
            Some(files) => Ok(files),
            None => Ok(GitFiles::new()),
        }
    }

    #[instrument(skip_all)]
    pub fn commit(&self, cli_args: &Cli, new_version: &Version) -> miette::Result<()> {
        let mut git = self.command(cli_args.suppress.includes_git());
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

        let _stdout = match Process::Output.run(git)? {
            ProcessOutput::Output(output) => output.stdout(),
            _ => unreachable!(),
        };
        self.dirty_files().context("After Commit")?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub fn tag(
        &self,
        cli_args: &Cli,
        version: &Version,
        args: Option<Vec<&str>>,
    ) -> miette::Result<()> {
        let mut git = self.command(cli_args.suppress.includes_git());
        git.arg("tag");
        if let Some(a) = args {
            git.args(a);
        }
        git.args([&self.generate_tag(version)]);
        let output = match Process::Output.run(git)? {
            ProcessOutput::Output(output) => output,
            _ => unreachable!(),
        };
        if !output.status.success() {
            tracing::debug!("stderr: {}", output.stderr());
            bail!("Failed to tag repository.")
        }
        Ok(())
    }

    #[instrument(skip_all)]
    pub fn generate_tag(&self, version: impl Display) -> String {
        let tag = version.to_string();
        debug! {"Tag: {tag}"};
        tag
    }

    /// Pushed just the tag to the remotes
    #[instrument(skip_all, fields(dry_run))]
    pub fn push(
        &self,
        cli_args: &Cli,
        version: &Version,
    ) -> miette::Result<Vec<(Task, Option<Child>)>> {
        current_span!().record("dry_run", cli_args.dry_run());
        let tag_string = String::from("tags/") + &self.generate_tag(version);
        let join = self
            .remotes()?
            .iter()
            .map(|remote| {
                let task = Task::GitPush {
                    tag: tag_string.clone(),
                    remote: remote.into(),
                    branch: Branch::Current, // TODO: Set to branch
                };
                info!("Pushing to remote: {remote}");
                let mut git_push = self.command(cli_args.suppress.includes_git());
                git_push.arg("push");
                if cli_args.dry_run() {
                    git_push.arg("--dry-run");
                }
                git_push.args([remote.as_str(), &tag_string, "--porcelain"]);
                tracing::debug!("Running: {:?}", git_push);
                let child = match Process::Spawn.run(git_push) {
                    Ok(ProcessOutput::Child(child)) => Ok(child),
                    Err(e) => Err(e),
                    _ => unreachable!(),
                };
                (task, child)
            })
            .collect::<Vec<_>>();
        let mut ret = vec![];

        for (t, c) in join {
            ret.push((t, Some(c?)));
        }

        Ok(ret)
    }

    /// Returns a list of remotes for the current branch.
    ///
    /// Returns an error if the list is empty
    #[instrument(skip_all)]
    pub fn remotes(&self) -> miette::Result<Vec<String>> {
        let mut git = self.command(true);
        git.args(["remote"]);

        let stdout = match Process::Output.run(git)? {
            ProcessOutput::Output(output) => output.stdout(),
            _ => unreachable!(),
        };

        let remotes: Vec<String> = stdout.lines().map(String::from).collect();

        let mut branch_remotes = IndexSet::new();

        for line in self.branch(vec!["--remotes"])?.lines() {
            let valid_remote = match line.split_once('/') {
                Some((remote, _branch)) => remote.trim().to_string(),
                None => {
                    warn!("Ensure you only run command on a branch with a remote.");
                    bail!(
                        help = "Use the branch flag to change to valid branch",
                        "Failed to find remote for current branch."
                    )
                }
            };
            assert!(remotes.contains(&valid_remote));

            branch_remotes.insert(valid_remote);
        }
        info!("Remotes: {:?}", branch_remotes);

        assert!(!branch_remotes.is_empty());
        assert!(remotes.len() >= branch_remotes.len());
        if branch_remotes.is_empty() {
            warn!("Ensure you only run command on a branch with a remote.");
            bail!("Failed to find remote for current branch.")
        }
        Ok(branch_remotes.into_iter().collect())
    }

    /// Runs `git branch` with any additional arguments.
    ///
    /// ## Errors
    ///
    /// This function will return an error if the [Command] fails.
    #[instrument(skip_all)]
    pub fn branch(&self, args: Vec<&str>) -> miette::Result<String> {
        let mut git = self.command(true);
        git.arg("branch");
        args.iter().for_each(|&arg| {
            git.arg(arg);
        });
        let stdout = match Process::Output.run(git)? {
            ProcessOutput::Output(output) => output.stdout(),
            _ => unreachable!(),
        };
        Ok(stdout)
    }

    #[instrument(skip_all)]
    pub fn current_branch(&self) -> Result<Branch> {
        // Determine current branch to return.
        let mut cmd = self.command(false);
        cmd.args(["branch", "--show-current"]);
        cmd.stdout(Stdio::piped());
        let current_branch = match Process::Output.run(cmd) {
            Ok(output) => match output {
                ProcessOutput::Output(b) => {
                    if b.status.success() {
                        b.stdout().trim_end().to_string()
                    } else {
                        miette::bail!(
                            help = "Failed to run 'git branch --show-current'",
                            "{}",
                            b.stderr()
                        );
                    }
                }
                _ => unreachable!(),
            },
            Err(e) => Err(e.wrap_err("Failed to run 'git branch --show-current'"))?,
        };
        Ok(Branch::Named {
            local: current_branch,
        })
    }

    #[allow(unreachable_code, unused_variables)]
    #[instrument(skip_all, fields(from, to, stash_revert_required))]
    pub fn checkout(
        &self,
        cli_args: &Cli,
        branch: Branch,
        stash_state: Stash,
    ) -> Result<(Branch, Stash)> {
        let current_branch = self.current_branch()?;

        let span = current_span!();
        span.record("from", current_branch.as_ref());
        span.record("to", branch.as_ref());

        tracing::debug!("Switch to {:?}", current_branch);
        unimplemented!("");

        // Check if need to stash.
        // #46
        let mut revert_stash = Stash::Dont;
        if stash_state.is_stash() {
            revert_stash = self.stash(cli_args.suppress, stash_state)?;
        }

        // Changing branch
        let mut cmd = self.command(cli_args.suppress.includes_git());

        if let Branch::Named { local } = &branch {
            cmd.args(["checkout", local.as_ref()]);
        } else {
            bail!("Can't change branch to current branch.")
        };

        let output = match Process::Output
            .run(cmd)
            .context(format!("Failed to run: git checkout {}", &branch))?
        {
            ProcessOutput::Output(output) => output,
            _ => unreachable!(),
        };

        if !output.status.success() {
            miette::bail!(
                help = "Failed to run 'git branch --show-current'",
                "{}",
                output.stderr()
            );
        }

        // #46
        if stash_state.is_unstash() {
            revert_stash = self.stash(cli_args.suppress, stash_state)?;
        }

        Ok((current_branch, revert_stash))
    }

    pub fn stash(&self, suppress: Suppress, state: Stash) -> Result<Stash> {
        // TODO: use `git stash {create, store, apply, drop}`
        // TODO: Ensure no dirty files after stash.
        let files = self.dirty_files()?;
        let mut ret_stash: Stash = state;

        let mut git = self.command(suppress.includes_git());
        git.arg("stash");

        match state {
            Stash::Stash => {
                git.arg("pop");
                ret_stash = Stash::Unstash
            }
            Stash::Unstash => {
                if !files.is_empty() {
                    git.arg("push");
                    ret_stash = Stash::Stash
                }
            }
            Stash::Dont => return Ok(state),
        };
        let command = Process::display_command(&git);
        let run = Process::Output.run(git)?;

        let output = run.as_output().unwrap();
        if !output.status.success() {
            miette::bail!(
                help = format!("Failed to run '{}'", command),
                "{}",
                output.stderr()
            );
        };
        Ok(ret_stash)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Hash)]
pub enum Stash {
    /// Run git stash push
    #[default]
    Stash,
    /// Run git stash pop
    Unstash,
    /// Don't run
    Dont,
}

impl Stash {
    /// Returns `true` if the stash is [`Stash`].
    ///
    /// [`Stash`]: Stash::Stash
    #[must_use]
    pub fn is_stash(&self) -> bool {
        matches!(self, Self::Stash)
    }

    /// Returns `true` if the stash is [`Unstash`].
    ///
    /// [`Unstash`]: Stash::Unstash
    pub fn revert_required(&self) -> bool {
        self.is_unstash()
    }

    /// Returns `true` if the stash is [`Unstash`].
    ///
    /// [`Unstash`]: Stash::Unstash
    #[must_use]
    pub fn is_unstash(&self) -> bool {
        matches!(self, Self::Unstash)
    }
}
