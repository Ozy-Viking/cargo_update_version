# Cargo Update Version

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
  -h, --help                  Print help

Version Change (Choose one):
  -p, --patch        Increment the version by 1 patch level. [default]
  -m, --minor        Increment the version by 1 minor level.
  -M, --major        Increment the version by 1 major level.
  -s, --set <0.1.1>  Set the version using valid semver.
```
