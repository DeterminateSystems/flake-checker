# Nix flake dependency check

:warning: The Nix Installer Action scanned your `flake.lock` and discovered a few issues that we recommend looking into.

## Non-supported Git branches for Nixpkgs

* The `nixpkgs` input uses the `this-should-fail` branch

<details>
<summary>What to do :toolbox:</summary>
Use one of these branches instead:

* `nixos-22.11`
* `nixos-22.11-small`
* `nixos-unstable`
* `nixos-unstable-small`
* `nixpkgs-22.11-darwin`
* `nixpkgs-unstable`

Here's an example:

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
}
```
</details>

<details>
<summary>Why it's important to use supported branches :books:</summary>
<a href="https://zero-to-nix.com/concepts/nixos">NixOS</a>'s release branches stop receiving updates roughly 7 months after release and then gradually become more and more insecure over time.
Non-release branches receive unpredictable updates and should be avoided as dependencies.
Release branches are also certain to have good <a href="https://zero-to-nix.com/concepts/caching">binary cache</a> coverage, which other branches can't promise.

</details>

## Outdated Nixpkgs dependencies

* The `nixpkgs` input is **40** days old

The maximum recommended age is **30** days.

<details>
<summary>What to do :toolbox:</summary>
Use the [`update-flake-lock`][flake-lock-action] GitHub Action to automate updates:

```yaml
steps:
  - name: Automatically update flake.lock
    uses: DeterminateSystems/update-flake-lock
    with:
      pr-title: "Update flake.lock"        # PR title
      pr-labels: [dependencies, automated] # PR labels
```
</details>

<details>
<summary>Why it's important to keep Nix dependencies up to date :books:</summary>
<a href="https://github.com/NixOS/nixpkgs">Nixpkgs</a> receives a continuous stream of security patches to keep your software and systems secure.
Using outdated revisions of Nixpkgs can inadvertently expose you to software security risks that have been resolved in more recent releases.

</details>

## Non-upstream Nixpkgs dependencies

* The `nixpkgs` input has `bitcoin-miner-org` as an owner rather than `NixOS`

<details>
<summary>What to do :toolbox:</summary>
Use a Nixpkgs dependency from the [`NixOS`][nixos] org.
Here's an example:

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs";
}
```

If you need a customized version of Nixpkgs, we recommend that you use [overlays] and per-package [overrides].
</details>

<details>
<summary>Why it's important to use upstream Nixpkgs :books:</summary>
We don't recommend using forked or re-exported versions of Nixpkgs.
While this may be convenient in some cases, it can introduce unexpected behaviors and unwanted security risks.
While <a href="https://github.com/NixOS/nixpkgs">upstream Nixpkgs</a> isn't bulletproof&mdash;nothing in software is!&mdash;it has a wide range of security measures in place, most notably continuous integration testing with <a href="https://hydra.nixos.org/">Hydra</a>, that mitigate a great deal of supply chain risk.

</details>

[flake-lock-action]: https://github.com/determinateSystems/update-flake-lock
[nixos]: https://github.com/nixos
[overlays]: https://nixos.wiki/wiki/Overlays
[overrides]: https://ryantm.github.io/nixpkgs/using/overrides
# Nix flake dependency check

:check: Your `flake.lock` has a clean bill of health.
