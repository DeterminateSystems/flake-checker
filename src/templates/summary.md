Scanning your `flake.lock` has turned up a few issues we recommend looking into:

Type | Description
:----|:-----------
{{#each issues}}
`{{ this.kind }}` {{#if (eq this.kind "outdated")}}:clock:{{/if}}{{#if (eq this.kind "disallowed")}}:x:{{/if}}{{#if (eq this.kind "disallowed")}}:branch:{{/if}} | {{{ this.message }}}
{{/each}}

{{#if has_disallowed}}
<details>
  <summary>Why using supported branches is important</summary>
  Insert info here.
</details>
{{/if}}

{{#if has_outdated}}
<details>
  <summary>Why keeping Nix dependencies up to date is important</summary>
  Insert info here.
</details>
{{/if}}

{{#if has_non_upstream}}
<details>
  <summary>Why using upstream Nixpkgs is important</summary>
  Insert info here.
</details>
{{/if}}
