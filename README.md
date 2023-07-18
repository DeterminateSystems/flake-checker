# Nix Flake Checker

**Nix Flake Checker** is a tool from [Determinate Systems][detsys] that performs "health" checks on the [`flake.lock`][lockfile] files in your [flake][flakes]-powered Nix projects.
Its goal is to help your Nix projects stay on recent and supported versions of [Nixpkgs].

To run the checker in the root of a Nix project:

```shell
nix run github:DeterminateSystems/flake-checker

# Or point to an explicit path for flake.lock
nix run github:DeterminateSystems/flake-checker /path/to/flake.lock
```

Nix Flake Checker looks at your `flake.lock`'s root-level [Nixpkgs] inputs and checks that:

- Any explicit Nixpkgs Git refs are in this list:
  - `nixos-22.11`
  - `nixos-22.11-small`
  - `nixos-23.05`
  - `nixos-23.05-small`
  - `nixos-unstable`
  - `nixos-unstable-small`
  - `nixpkgs-22.11-darwin`
  - `nixpkgs-23.05-darwin`
  - `nixpkgs-unstable`
- Any Nixpkgs dependencies are less than 30 days old
- Any Nixpkgs dependencies have the [`NixOS`][nixos-org] org as the GitHub owner (and thus that the dependency isn't a fork or non-upstream variant)

If you're running it locally, Nix Flake Checker reports any issues via text output in your terminal.
But you can also use Nix Flake Checker [in CI](#the-flake-checker-action).

## The Nix Flake Checker Action

You can automate Nix Flake Checker by adding Determinate Systems' [Nix Flake Checker Action][action] to your GitHub Actions workflows:

```yaml
checks:
  steps:
    - uses: actions/checkout@v3
    - name: Check Nix flake Nixpkgs inputs
      uses: DeterminateSystems/flake-checker-action@main
```

When run in GitHub Actions, Nix Flake Checker always exits with a status code of 0 by default&mdash;and thus never fails your workflows&mdash;and reports its findings as a [Markdown summary][md].

## Telemetry

The goal of Nix Flake Checker is to help teams stay on recent and supported versions of Nixpkgs.
The flake checker collects a little bit of telemetry information to help us make that true.

Here is a table of the [telemetry data we collect][telemetry]:

| Field          | Use                                                                                                    |
| -------------- | ------------------------------------------------------------------------------------------------------ |
| `distinct_id`  | An opaque string that represents your project, by sha256 hashing repository and organization details.  |
| `version`      | The version of Nix Flake Checker.                                                                      |
| `is_ci`        | Whether the checker is being used in CI (GitHub Actions).                                              |
| `disallowed`   | The number of inputs using unsupported branches of Nixpkgs.                                            |
| `outdated`     | The number of inputs using outdated versions of Nixpkgs.                                               |
| `non_upstream` | The number of inputs using forks of Nixpkgs.                                                           |

To disable diagnostic reporting, set the diagnostics URL to an empty string by passing `--no-telemetry` or setting `FLAKE_CHECKER_NO_TELEMETRY=true`.

You can read the full privacy policy for [Determinate Systems][detsys], the creators of this tool and the [Determinate Nix Installer][installer], [here][privacy].

## Rust library

The Nix Flake Checker is written in [Rust].
This repo exposes a [`parse-flake-lock`](./parse-flake-lock) crate that you can use to parse [`flake.lock` files][lockfile] in your own Rust projects.
To add that dependency:

```toml
[dependencies]
parse-flake-lock = { git = "https://github.com/DeterminateSystems/flake-checker", branch = "main" }
```

Here's an example usage:

```rust
use std::path::Path;

use parse_flake_lock::{FlakeLock, FlakeLockParseError};

fn main() -> Result<(), FlakeLockParseError> {
    let flake_lock = FlakeLock::new(Path::new("flake.lock"))?;
    println!("flake.lock info:");
    println!("version: {version}", version=flake_lock.version);
    println!("root node: {root:?}", root=flake_lock.root);
    println!("all nodes: {nodes:?}", nodes=flake_lock.nodes);

    Ok(())
}
```

The `parse-flake-lock` crate doesn't yet exhaustively parse all input node types, instead using a "fallthrough" mechanism that parses input types that don't yet have explicit struct definitions to a [`serde_json::value::Value`][val].
If you'd like to help make the parser more exhaustive, [pull requests][prs] are quite welcome.

[action]: https://github.com/DeterminateSystems/flake-checker-action
[detsys]: https://determinate.systems
[flakes]: https://zero-to-nix.com/concepts/flakes
[install]: https://zero-to-nix.com/start/install
[installer]: https://github.com/DeterminateSystems/nix-installer
[lockfile]: https://zero-to-nix.com/concepts/flakes#lockfile
[md]: https://github.blog/2022-05-09-supercharging-github-actions-with-job-summaries
[nixos-org]: https://github.com/NixOS
[nixpkgs]: https://github.com/NixOS/nixpkgs
[privacy]: https://determinate.systems/privacy
[prs]: /pulls
[rust]: https://rust-lang.org
[telemetry]: https://github.com/DeterminateSystems/nix-flake-checker/blob/main/src/telemetry.rs#L29-L43
[val]: https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html
