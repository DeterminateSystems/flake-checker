:warning: Scanning your `flake.lock` has turned up a few issues we recommend looking into:

{{#if has_disallowed}}
---

We found some non-supported branches:

{{#each disallowed}}
* `{{this.details.input}}` uses ref `{{this.details.ref}}`
{{/each}}

Use one of these instead:

{{{supported_ref_names}}}

<details>
  <summary>Why using supported branches is important</summary>
  Insert info here.
</details>
{{/if}}

{{#if has_outdated}}
---

We found some outdated Nixpkgs dependencies:

{{#each outdated}}
* `{{this.details.input}}` is **{{this.details.num_days_old}}** days old
{{/each}}

The maximum age is **{{max_days}}** days.

<details>
  <summary>Why keeping Nix dependencies up to date is important</summary>
  Insert info here.
</details>

{{#if has_non_upstream }}---{{/if}}
{{/if}}

{{#if has_non_upstream}}
---

We found some non-upstream Nixpkgs dependencies:

{{#each non_upstream}}
* `{{this.details.input}}` has `{{this.details.owner}}` as an owner rather than `NixOS`
{{/each}}

<details>
  <summary>Why using upstream Nixpkgs is important</summary>
  Insert info here.
</details>
{{/if}}
