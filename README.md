# flake-checker

```shell
nix build

./result/bin/flake-checker

# Or point to an explicit path
./result/bin/flake-checker /path/to/flake.lock
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
