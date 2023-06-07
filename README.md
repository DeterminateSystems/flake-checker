# flake-checker

```shell
nix run github:DeterminateSystems/flake-checker

# Or point to an explicit path
nix run github:DeterminateSystems/flake-checker /path/to/flake.lock
```

Currently performs two checks:

- Checks that any explicit Nixpkgs Git refs are in this list:
  - `nixos-22.11`
  - `nixos-22.11-small`
  - `nixos-unstable`
  - `nixos-unstable-small`
  - `nixpkgs-22.11-darwin`
  - `nixpkgs-unstable`
- Checks that any Nixpkgs dependencies are less than 30 days old

### Telemetry

The goal of the Determinate Flake Checker is to help teams stay on recent and supported versions of Nixpkgs.
The flake checker collects a little bit of telemetry information to help us make that true.

Here is a table of the [telemetry data we collect][diagnosticdata]:

| Field          | Use                                                                                                    |
| -------------- | ------------------------------------------------------------------------------------------------------ |
| `distinct_id`  | An opaque string which represents your project, by sha256 hashing repository and organization details. |
| `version`      | The version of the Determinate Flake Checker.                                                          |
| `is_ci`        | Whether the checker is being used in CI (e.g. GitHub Actions).                                         |
| `disallowed`   | The number of inputs using unsupported branches of Nixpkgs.                                            |
| `outdated`     | The number of inputs using outdated versions of Nixpkgs.                                               |
| `non_upstream` | The number of inputs using forks of Nixpkgs.                                                           |

To disable diagnostic reporting, set the diagnostics URL to an empty string by passing `--no-telemetry` or setting `FLAKE_CHECKER_NO_TELEMETRY=true`.

You can read the full privacy policy for [Determinate Systems][detsys], the creators of the Determinate Nix Installer, [here][privacy].

[detsys]: https://determinate.systems/
[diagnosticdata]: https://github.com/DeterminateSystems/nix-flake-checker/blob/main/src/telemetry.rs#L29-L43
[privacy]: https://determinate.systems/privacy
