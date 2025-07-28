# Cargo Update Version

![GitHub License](https://img.shields.io/github/license/ozy-viking/cargo_update_version?style=for-the-badge&link=https%3A%2F%2Fopensource.org%2Flicense%2Fmit)
![Crates.io Version](https://img.shields.io/crates/v/cargo-uv?style=for-the-badge&logo=rust&color=blue&link=https%3A%2F%2Fcrates.io%2Fcrates%2Fcargo-uv)

## Install

### Using Cargo Binstall

```shell
cargo binstall cargo-uv
```

### Using Cargo

```shell
cargo install cargo-uv
```

This is a work in progress.

## Usage

```text
A simple Cargo tool for updating the version in your project.

Usage: cargo uv [OPTIONS] [ACTION] [SET_VERSION]

Arguments:
  [ACTION]       Action to affect the package version [default: patch] [possible values: patch, minor, major, set, print, tree]
  [SET_VERSION]  New version to set. Ignored if action isn't set

Options:
      --pre <PRE>            Sets the pre-release segment for the new version.
      --build <BUILD>        Sets the build metadata for the new version.
  -Q, --suppress <SUPPRESS>  What to suppress from stdout [default: none] [possible values: none, git, cargo, all]
  -n, --allow-dirty          Allows program to work in a dirty repo.
  -f, --force-version        Bypass version bump checks.
  -d, --dry-run              Allows git tag to occur in a dirty repo.
      --color <WHEN>         Controls when to use color [default: auto] [possible values: auto, always, never]
  -v, --verbose...           Increase logging verbosity
  -q, --quiet...             Decrease logging verbosity
  -h, --help                 Print help (see more with '--help')
  -V, --version              Print version

Cargo:
  -c, --cargo-publish         Runs the `cargo publish`
      --no-verify             adds 'no_verify' to cargo publish command
      --manifest-path <PATH>  Path to Cargo.toml. All commands run as if they run in the the directory of the Cargo.toml set

Git:
  -t, --git-tag            Create a git tag.
      --git-push           Push tag to the branch's remote repositries.
  -m, --message <MESSAGE>  Message for git commit. Default to git tag.
      --force-git          Pass force into all git operations.

Package Selection:
  -p, --package <SPEC>     Package to process (see `cargo help pkgid`)
  -x, --exclude <SPEC>     Exclude packages from being processed
      --workspace          Process all packages in the workspace [aliases: --all]
      --workspace-package  Process workspace.package.version [aliases: --ws]
      --default-members    Process only default workspace members
```
