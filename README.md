# flake-checker

```shell
nix run . -- --path ./flake.lock
```

Currently performs two checks:

* Checks that any explicit Nixpkgs Git refs are in this list:
  * `nixos-22.11`
  * `nixos-22.11-small`
  * `nixos-unstable`
  * `nixos-unstable-small`
  * `nixpkgs-22.11-darwin`
  * `nixpkgs-unstable`
* Checks that any Nixpkgs dependencies are less than 30 days old
