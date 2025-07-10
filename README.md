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
  [ACTION]       Action to affect the package version [default: patch] [possible values: patch, minor, major, set, print]
  [SET_VERSION]  New version to set. Ignored if action isn't set

Options:
      --pre <PRE>             Sets the pre-release segment for the new version.
      --build <BUILD>         Sets the build metadata for the new version.
  -n, --allow-dirty           Allows program to work in a dirty repo.
      --manifest-path <PATH>  Path to Cargo.toml
  -f, --force-version         Bypass version bump checks.
  -d, --dry-run               Allows git tag to occur in a dirty repo.
      --color <WHEN>          Controls when to use color [default: auto] [possible values: auto, always, never]
  -v, --verbose...            Increase logging verbosity
  -q, --quiet...              Decrease logging verbosity
  -h, --help                  Print help (see more with '--help')
  -V, --version               Print version

Cargo:
  -c, --cargo-publish   Runs the `cargo publish`
  -Q, --supress-stdout  Suppresses stdout out from cargo commands run
      --no-verify       adds 'no_verify' to cargo publish command

Git:
  -t, --git-tag            Create a git tag.
      --git-push           Push tag to the branch's remote repositries.
  -m, --message <MESSAGE>  Message for git commit. Default to git tag.
      --force-git          Pass force into all git operations.
      --git-supress        Suppresses stdout out from git commands run
```
