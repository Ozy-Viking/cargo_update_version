# Cargo Update Version

![GitHub License](https://img.shields.io/github/license/ozy-viking/cargo_update_version?style=for-the-badge&link=https%3A%2F%2Fopensource.org%2Flicense%2Fmit)
![Crates.io Version](https://img.shields.io/crates/v/cargo-uv?style=for-the-badge&logo=rust&color=blue&link=https%3A%2F%2Fcrates.io%2Fcrates%2Fcargo-uv)

This is a work in progress.

## Usage

1. Bump version 1 patch i.e. `0.1.0 -> 0.1.1`

    ```bash
    cargo uv 
    ```

2. Bump version 1 minor i.e. `0.1.3 -> 0.2.0`

    ```bash
    cargo uv -m
    ```

3. Bump version 1 major i.e. `0.2.3 -> 1.0.0`

    ```bash
    cargo uv -M
    ```

4. Set version

    ```bash
    cargo uv -s 0.2.1
    ```

5. Bump version 1 patch and set git tag i.e. `0.1.0 -> 0.1.1`

    ```bash
    cargo uv -t
    cargo uv -tc "Custom Git message for commit"
    ```

6. Print version to stdout

    ```bash
    cargo uv -V
    cargo uv --print
    ```

7. Bump version, tag, push and publish

    ```bash
    cargo uv -t --push --publish
    ```

```text
Usage: cargo uv [OPTIONS]

Options:
  -v, --verbose...            Increase logging verbosity
  -q, --quiet...              Decrease logging verbosity
  -P, --manifest-path <Path>  Path to the Cargo.toml file.
  -f, --force-version         Force version bump, this will disregard all version checks.
  -t, --git-tag               Will run git tag as well.
  -a, --allow-dirty           Allows git tag to occur in a dirty repo.
  -c, --message <message>     Message for git commit. Defaults to new version number.
  -V, --version               Prints the current version of your project then exits.
  -n, --dry-run               Does a dry-run, will create a tag but then deletes it.
      --push                  Pushes the tag to all remotes of the current branch not just origin.
      --publish               Runs cargo publish. Allow dirty is required here.
  -h, --help                  Print help (see more with '--help')

Version Change (Choose one):
  -p, --patch        Increment the version by 1 patch level. [default selection]
  -m, --minor        Increment the version by 1 minor level.
  -M, --major        Increment the version by 1 major level.
  -s, --set <0.3.2>  Set the version using valid semver.
```
