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

5. TODO: Bump version 1 patch and set git tag i.e. `0.1.0 -> 0.1.1`

    ```bash
    cargo uv -t
    ```

6. TODO: Print version to stdout

    ```bash
    cargo uv -p
    ```
