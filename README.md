# Nix Flake Checker

[![FlakeHub](https://img.shields.io/endpoint?url=https://flakehub.com/f/DeterminateSystems/flake-checker/badge)](https://flakehub.com/flake/DeterminateSystems/flake-checker)

**Nix Flake Checker** is a tool from [Determinate Systems][detsys] that performs "health" checks on the [`flake.lock`][lockfile] files in your [flake][flakes]-powered Nix projects.
Its goal is to help your Nix projects stay on recent and supported versions of [Nixpkgs].

To run the checker in the root of a Nix project:

```shell
nix run github:DeterminateSystems/flake-checker

# Or point to an explicit path for flake.lock
nix run github:DeterminateSystems/flake-checker /path/to/flake.lock
```

Nix Flake Checker looks at your `flake.lock`'s root-level [Nixpkgs] inputs.
There are two ways to express flake policies:

- Via [config parameters](#parameters).
- Via [policy conditions](#policy-conditions) using [Common Expression Language][cel] (CEL).

If you're running it locally, Nix Flake Checker reports any issues via text output in your terminal.
But you can also use Nix Flake Checker [in CI](#the-flake-checker-action).

## Supported branches

At any given time, [Nixpkgs] has a bounded set of branches that are considered _supported_.
The current list, with their statuses:

- `nixos-25.05`
- `nixos-25.05-small`
- `nixos-unstable`
- `nixos-unstable-small`
- `nixpkgs-25.05-darwin`
- `nixpkgs-unstable`

## Parameters

By default, Flake Checker verifies that:

- Any explicit Nixpkgs Git refs are in the [supported list](#supported-branches).
- Any Nixpkgs dependencies are less than 30 days old.
- Any Nixpkgs dependencies have the [`NixOS`][nixos-org] org as the GitHub owner (and thus that the dependency isn't a fork or non-upstream variant).

You can adjust this behavior via configuration (all are enabled by default but you can disable them):

| Flag                | Environment variable                | Action                                                     | Default |
| :------------------ | :---------------------------------- | :--------------------------------------------------------- | :------ |
| `--check-outdated`  | `NIX_FLAKE_CHECKER_CHECK_OUTDATED`  | Check for outdated Nixpkgs inputs                          | `true`  |
| `--check-owner`     | `NIX_FLAKE_CHECKER_CHECK_OWNER`     | Check that Nixpkgs inputs have `NixOS` as the GitHub owner | `true`  |
| `--check-supported` | `NIX_FLAKE_CHECKER_CHECK_SUPPORTED` | Check that Git refs for Nixpkgs inputs are supported       | `true`  |

## Policy conditions

You can apply a CEL condition to your flake using the `--condition` flag.
Here's an example:

```shell
flake-checker --condition "has(numDaysOld) && numDaysOld < 365"
```

This would check that each Nixpkgs input in your `flake.lock` is less than 365 days old.
These variables are available in each condition:

| Variable        | Description                                                                                                                              |
| :-------------- | :--------------------------------------------------------------------------------------------------------------------------------------- |
| `gitRef`        | The Git reference of the input.                                                                                                          |
| `numDaysOld`    | The number of days old the input is.                                                                                                     |
| `owner`         | The input's owner (if a GitHub input).                                                                                                   |
| `supportedRefs` | A list of [supported Git refs](#supported-branches) (all are branch names).                                                              |
| `refStatuses`   | A map. Each key is a branch name. Each value is a branch status (`"rolling"`, `"beta"`, `"stable"`, `"deprecated"` or `"unmaintained"`). |

We recommend a condition _at least_ this stringent:

```ruby
supportedRefs.contains(gitRef) && (has(numDaysOld) && numDaysOld < 30) && owner == 'NixOS'
```

Note that not all Nixpkgs inputs have a `numDaysOld` field, so make sure to ensure that that field exists when checking for the number of days.

Here are some other example conditions:

```ruby
# Updated in the last two weeks
supportedRefs.contains(gitRef) && (has(numDaysOld) && numDaysOld < 14) && owner == 'NixOS'

# Check for most recent stable Nixpkgs
gitRef.contains("24.05")
```

## The Nix Flake Checker Action

You can automate Nix Flake Checker by adding Determinate Systems' [Nix Flake Checker Action][action] to your GitHub Actions workflows:

```yaml
checks:
  steps:
    - uses: actions/checkout@v4
    - name: Check Nix flake Nixpkgs inputs
      uses: DeterminateSystems/flake-checker-action@main
```

When run in GitHub Actions, Nix Flake Checker always exits with a status code of 0 by default&mdash;and thus never fails your workflows&mdash;and reports its findings as a [Markdown summary][md].

## Telemetry

The goal of Nix Flake Checker is to help teams stay on recent and supported versions of Nixpkgs.
The flake checker collects a little bit of telemetry information to help us make that true.

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
[cel]: https://cel.dev
[detsys]: https://determinate.systems
[flakes]: https://zero-to-nix.com/concepts/flakes
[install]: https://zero-to-nix.com/start/install
[installer]: https://github.com/DeterminateSystems/nix-installer
[lockfile]: https://zero-to-nix.com/concepts/flakes#lockfile
[md]: https://github.blog/2022-05-09-supercharging-github-actions-with-job-summaries
[nixos-org]: https://github.com/NixOS
[nixpkgs]: https://github.com/NixOS/nixpkgs
[privacy]: https://determinate.systems/policies/privacy
[prs]: /pulls
[rust]: https://rust-lang.org
[telemetry]: https://github.com/DeterminateSystems/nix-flake-checker/blob/main/src/telemetry.rs#L29-L43
[val]: https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html
