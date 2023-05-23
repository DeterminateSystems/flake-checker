# Nix flake dependency check

:warning: Scanning your `flake.lock` has turned up a few issues we recommend looking into.

{{#if has_disallowed}}
## Non-supported branches

{{#each disallowed}}
* `{{this.details.input}}` uses ref `{{this.details.ref}}`
{{/each}}

<details>
<summary>What to do :hammer:</summary>
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
<summary>Why using supported branches is important</summary>
{{{supported_refs_explainer}}}
</details>
{{/if}}

{{#if has_outdated}}
## Outdated Nixpkgs dependencies

{{#each outdated}}
* `{{this.details.input}}` is **{{this.details.num_days_old}}** days old
{{/each}}

The maximum recommended age is **{{max_days}}** days.

<details>
<summary>What to do :hammer:</summary>

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
<summary>Why keeping Nix dependencies up to date is important</summary>
{{{ outdated_deps_explainer }}}
</details>
{{/if}}

{{#if has_non_upstream}}
## Non-upstream Nixpkgs dependencies

{{#each non_upstream}}
* `{{this.details.input}}` has `{{this.details.owner}}` as an owner rather than `NixOS`
{{/each}}

<details>
<summary>What to do :hammer:</summary>
Use a Nixpkgs dependency from the [`NixOS`][nixoks] org.
Here's an example:

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs";
}
```
</details>

<details>
  <summary>Why using upstream Nixpkgs is important</summary>
  {{{ upstream_nixpkgs_explainer }}}
</details>
{{/if}}

[flake-lock-action]: https://github.com/determinateSystems/update-flake-lock
[nixos]: https://github.com/nixos
