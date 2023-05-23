# Nix flake dependency check

:warning: The Nix Installer Action scanned your `flake.lock` and discovered a few issues that we recommend looking into.

{{#if has_disallowed}}
## Non-supported Git branches for Nixpkgs

{{#each disallowed}}
* The `{{this.details.input}}` input uses the `{{this.details.ref}}` branch
{{/each}}

<details>
<summary>What to do :toolbox:</summary>
Use one of these branches instead:

{{{supported_ref_names}}}

Here's an example:

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
}
```
</details>

<details>
<summary>Why it's important to use supported branches :books:</summary>
{{{supported_refs_explainer}}}
</details>
{{/if}}

{{#if has_outdated}}
## Outdated Nixpkgs dependencies

{{#each outdated}}
* The `{{this.details.input}}` input is **{{this.details.num_days_old}}** days old
{{/each}}

The maximum recommended age is **{{max_days}}** days.

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
{{{ outdated_deps_explainer }}}
</details>
{{/if}}

{{#if has_non_upstream}}
## Non-upstream Nixpkgs dependencies

{{#each non_upstream}}
* The `{{this.details.input}}` input has `{{this.details.owner}}` as an owner rather than `NixOS`
{{/each}}

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
{{{ upstream_nixpkgs_explainer }}}
</details>
{{/if}}

[flake-lock-action]: https://github.com/determinateSystems/update-flake-lock
[nixos]: https://github.com/nixos
[overlays]: https://nixos.wiki/wiki/Overlays
[overrides]: https://ryantm.github.io/nixpkgs/using/overrides
