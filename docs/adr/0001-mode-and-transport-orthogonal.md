# Mode and transport format are orthogonal axes

ovq has two independent decisions to make on every invocation: which **mode** it runs in (query mode that filters files, or values mode that aggregates one property), and which **transport format** it emits (plain text, TSV with selected fields, or JSON). The CLI treats them as orthogonal — `--json` is not a query-only flag, it applies to both modes; `--fields` is a query-mode-only flag (values mode aggregates a single property, so per-file field selection has no meaning there).

The consequence that matters most for callers: when both `--json` and `--fields` are set, the JSON `frontmatter` object contains *only* the requested fields, not the full frontmatter. The transport format honours the same "give me only what I asked for" principle the rest of the CLI uses. This is the opposite of the original plan, which had `--fields` ignored when `--json` was set; we changed our minds because the unified principle (one output schema, requested fields included, unrequested fields omitted) is simpler to teach, simpler to test, and friendlier to consumers piping through `jq` who want narrow results.

Rejected: keeping `--fields` as query-mode-only and having `--json` always emit the full frontmatter. It made the two flags non-composable, which is surprising once you have both.
