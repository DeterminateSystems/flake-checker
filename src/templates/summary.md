Scanning your `flake.lock` has turned up a few issues we recommend looking into:

Type | Description
:----|:-----------
{{#each issues}}
`{{ this.kind }}` {{#if (eq this.kind "outdated")}}:clock:{{/if}}{{#if (eq this.kind "disallowed")}}:x:{{/if}} | {{{ this.message }}}
{{/each}}
