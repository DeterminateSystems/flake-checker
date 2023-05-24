# Nix flake dependency check

{{#if clean}}
✅ Your `flake.lock` has a clean bill of health.
{{/if}}
{{#if dirty}}
⚠️ The Nix Installer Action scanned your `flake.lock` and discovered a few issues that we recommend looking into.

{{#if has_disallowed}}
## Non-supported Git branches for Nixpkgs

{{#each disallowed}}
* The `{{this.details.input}}` input uses the `{{this.details.ref}}` branch
{{/each}}

<details>
<summary>What to do 🧰</summary>
<p>Use one of these branches instead:</p>

{{{supported_ref_names}}}

<p>Here's an example:</p>

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
}
```
</details>

<details>
<summary>Why it's important to use supported branches 📚</summary>
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
<summary>What to do 🧰</summary>
<p>Use the <a href="https://github.com/determinateSystems/update-flake-lock"><code>update-flake-lock</code></a>
GitHub Action to automate updates:</p>

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
<summary>Why it's important to keep Nix dependencies up to date 📚</summary>
{{{ outdated_deps_explainer }}}
</details>
{{/if}}

{{#if has_non_upstream}}
## Non-upstream Nixpkgs dependencies

{{#each non_upstream}}
* The `{{this.details.input}}` input has `{{this.details.owner}}` as an owner rather than `NixOS`
{{/each}}

<details>
<summary>What to do 🧰</summary>
<p>Use a Nixpkgs dependency from the <a href="https://github.com/nixos"><code>NixOS</code></a> org. Here's an example:</p>

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs";
}
```

<p>If you need a customized version of Nixpkgs, we recommend that you use
<a href="https://nixos.wiki/wiki/Overlays">overlays</a> and
per-package <a href="https://ryantm.github.io/nixpkgs/using/overrides">overrides</a>.</p>
</details>

<details>
<summary>Why it's important to use upstream Nixpkgs 📚</summary>
{{{ upstream_nixpkgs_explainer }}}
</details>
{{/if}}
{{/if}}
