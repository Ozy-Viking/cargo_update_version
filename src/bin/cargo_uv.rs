use std::env::args;

use cargo_uv::{
    Action, Cargo, Cli, FOOTER, GitBuilder, Packages, Result, Stash, Task, Tasks, VersionType,
    setup_tracing,
};
use clap::CommandFactory;
use miette::{Context, IntoDiagnostic, ensure, miette};
use rusty_viking::MietteDefaultConfig;

use clap::FromArgMatches;
fn main() -> Result<()> {
    // removes uv from from input
    // BUG: #27 Cli ignores any use of 'uv' in args.
    let input = args().filter(|a| a != "uv").collect::<Vec<_>>();
    MietteDefaultConfig::init_set_panic_hook(Some(FOOTER.into()))?;
    let mut cli = Cli::command();
    cli = cli.mut_arg("set_version", |a| a.required_if_eq("action", Action::Set));
    cli.set_bin_name("cargo uv");
    cli = cli.next_line_help(false);

    let mut args = Cli::from_arg_matches(&cli.get_matches_from(&input)).into_diagnostic()?;

    setup_tracing(&args)?;
    let root_dir = args.root_dir()?;
    let git = GitBuilder::new().root_directory(root_dir).build();

    let starting_branch = if args.git_branch().is_other() {
        Some(git.checkout(&args, args.git_branch(), Stash::Unstashed)?)
    } else {
        None
    };
    args.try_allow_dirty()?;

    let meta = args.get_metadata()?;
    let mut packages = Packages::from(meta);

    match args.action() {
        Action::Print => {
            let root_package = args.get_metadata()?.root_package().ok_or(miette::miette!(
                help = "Use the tree action if in a workspace without a root package.",
                "When printing, expected a root package."
            ))?;
            println!("{} {}", root_package.name, root_package.version);
            return Ok(());
        }
        Action::Tree => {
            println!("{}", packages.display_tree());
            return Ok(());
        }
        _ => (),
    }

    let (included, excluded) = args.workspace.partition_packages(&packages)?;

    ensure!(
        !included.is_empty(),
        help = "Check you are not excluding your root package without including others.",
        "No packages to modify. Excluded are: {:?}",
        excluded
            .iter()
            .map(|p| p.name().to_string())
            .collect::<Vec<_>>()
    );
    drop(excluded);

    // BUG: #40 Not using the the '--workspace-package' flag.
    let mut change_workspace_package_version = false;
    let mut tasks = Tasks::new();

    for package in included {
        if package.version_type() == VersionType::SetByWorkspace {
            change_workspace_package_version = true;
            tracing::info!(
                "Changing Workspace Package Version due to: {}",
                package.name()
            );
        } else {
            // TODO: Move to refs over cloning.
            let task = Task::from_action(
                args.action(),
                package.clone(),
                args.pre.clone(),
                args.build.clone(),
                args.set_version.clone(),
                args.force_version,
            )?;

            tasks.insert(task, None);
        }
    }

    if change_workspace_package_version {
        let task = match args.action() {
            Action::Pre | Action::Patch | Action::Minor | Action::Major => {
                Some(Task::BumpWorkspace {
                    bump: args.action(),
                    pre: args.pre.clone(),
                    build: args.build.clone(),
                    force: args.force_version,
                })
            }
            Action::Set => Some(Task::SetWorkspace {
                version: args.set_version.clone().ok_or(miette::miette!(
                    "Expected a new version for Task::from_action when action is Set"
                ))?,
            }),
            Action::Print | Action::Tree => None,
        };
        if let Some(t) = task {
            tasks.insert(t, None);
        }
    }
    // BUG: #43 Need to undo tasks on failure e.g. checkout, stash ...

    for mut task in tasks.version_change_tasks() {
        match &mut task {
            Task::Set {
                version: new_version,
                package,
            } => {
                package.set_version(new_version.clone())?;
                if !args.dry_run() {
                    package.write_cargo_file()?;
                }
            }
            Task::Bump {
                package,
                bump,
                pre,
                build,
                force,
            } => {
                package.bump_version(*bump, pre.clone(), build.clone(), *force)?;
                if !args.dry_run() {
                    package.write_cargo_file()?;
                }
            }
            Task::BumpWorkspace {
                bump,
                pre,
                build,
                force,
            } => {
                let workspace_package = packages.workspace_package().ok_or(miette!(
                    "1 or more packages are expecting there to be a workspace.package"
                ))?;
                Some(workspace_package.bump_version(*bump, pre.clone(), build.clone(), *force)?);
                if !args.dry_run() {
                    workspace_package.write_cargo_file()?;
                }
            }
            Task::SetWorkspace { version } => {
                let workspace_package = packages.workspace_package().ok_or(miette!(
                    "1 or more packages are expecting there to be a workspace.package"
                ))?;
                Some(workspace_package.set_version(version.clone())?);
                if !args.dry_run() {
                    workspace_package.write_cargo_file()?;
                }
            }
            _ => unreachable!(),
        }
        tracing::info!("Complete: {task}");
        tasks.complete_task(&task);
    }

    Cargo::generate_lockfile(&args)?;

    if args.git_tag() {
        tracing::info!("Generating git tag");

        let new_version = packages.root_version()?;

        git.add_cargo_files()?;
        git.commit(&args, &new_version)?; // BUG: #26 Not handling the case when the message is set.
        git.tag(&args, &new_version, None)?;
        if args.git_push() {
            let gpjh = git.push(&args, &new_version).context("git push")?;
            tasks.append(gpjh);
        }
        // BUG: #2 Deletes tag before push so push fails.
        if args.dry_run() {
            let task = Task::DeleteGitTag(new_version);
            tasks.insert(task, None);
        }
    }

    if args.cargo_publish() {
        tasks.insert(
            cargo_uv::Task::Publish,
            Some(Cargo::publish(&args).context("Cargo Publish")?),
        );
    }

    let tasks = match tasks.join_all() {
        Ok(ts) => ts,
        Err(e) => {
            tracing::warn!("Tasks errored, Completed tasks: {:?}", e.completed_tasks);
            tracing::warn!("Tasks with unknown status: {:?}", e.incomplete_tasks);
            return Err(e.into());
        }
    };

    if let Some(Task::DeleteGitTag(version)) = tasks.delete_tag() {
        let root_dir = args.root_dir()?;
        let git = GitBuilder::new().root_directory(root_dir).build();
        git.tag(&args, version, Some(vec!["--delete"]))?;
    };

    if let Some((branch, stash)) = starting_branch {
        git.checkout(&args, branch, stash)?;
    }
    Ok(())
}
