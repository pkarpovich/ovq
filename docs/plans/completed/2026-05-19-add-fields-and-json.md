# Add `--fields` and `--json` output flags

## Overview

ovq currently emits one filename per match in query mode. Almost every real use case wants follow-up data from the frontmatter of those matches: which `attribution` is on each Quote, which `rating` per Movie, which `status` per Task. Today that requires a second pass (read each file, awk the frontmatter, recombine), which is awkward in the shell and slow when the model is iterating.

This plan adds two flags:

- `--fields <list>` (comma-separated property names): for each matched file, emit a TSV row of `path\tfield1\tfield2\t...`. Composes with `awk`, `cut`, `column -t`, `sort`.
- `--json`: emit a JSON array of `{file, frontmatter}` objects. Always returns the full frontmatter (per design decision below), so consumers `jq` whatever they need.

Backwards compatibility: without these flags, output is unchanged (one path per line).

## Context (from discovery)

- Repo: `~/Projects/ovq`, Rust 2021, v0.2.0, ~1027 LOC across 8 source files.
- Files involved:
  - `src/main.rs` (CLI struct, dispatch) - add 2 flags, route to new formatter
  - `src/output.rs` (NEW) - formatters (plain, tsv-with-fields, json)
  - `Cargo.toml` - add `serde_json` dep
  - `README.md`, `CLAUDE.md` - doc the new flags
- Patterns to follow:
  - `clap` derive with `#[arg(long, help = "...")]`
  - `serde_yaml::Value` is what the query loop already hands around; serialize same to JSON via `serde_json`
  - Inline `#[cfg(test)] mod tests` per source file (see `src/values.rs:55+`, `src/query/eval.rs`)
  - ExitCode from `std::process::ExitCode`
- Recent: branch `feat/truthy-null-checks` already merged at 698cf53. `master` HEAD is 36cfa86 (v0.2.0 bump). No in-flight branches.

## Skills to invoke

- `rust-style` - all code in `src/` follows it (let-else early returns, for-loops over iterators where there is no combinator chain, shadowing over `_new` suffixes, newtypes, minimal comments)
- `rustdoc` - apply when adding `///` doc comments on the new public items in `output.rs` and on the new `Cli` flags

## Development Approach

- **Testing approach**: TDD (tests first)
- Complete each task fully before moving to the next
- Make small, focused changes
- **CRITICAL: every task MUST include new/updated tests** for code changes in that task
  - tests are not optional - they are a required part of the checklist
  - write unit tests for new functions/methods
  - tests cover both success and error scenarios
- **CRITICAL: all tests must pass before starting next task** - no exceptions
- **CRITICAL: update this plan file when scope changes during implementation**
- Run tests after each change
- Maintain backward compatibility - default output is unchanged

## Testing Strategy

- Unit tests inline per source file via `#[cfg(test)] mod tests`, matching existing project pattern.
- No e2e / UI tests in this project; CLI is exercised via `cargo run --` smoke tests at the end.

## Progress Tracking

- Mark completed items with `[x]` immediately when done
- Add newly discovered tasks with the right symbol
- Document issues/blockers with `WARN:` prefix
- Update plan if implementation deviates from original scope
- Keep plan in sync with actual work done

## Design decisions (after grill-with-docs session, 2026-05-19)

### Mode and transport format are orthogonal
See [ADR 0001](../adr/0001-mode-and-transport-orthogonal.md). `--json` applies to both query mode and values mode. `--fields` is query-mode only. `--values + --fields` is an error (exit 2). The CLI mode (query vs values) is decided by presence of `--values`; the transport format (plain / TSV / JSON) is decided by `--fields` / `--json`.

### Unified output schema, requested fields included, unrequested omitted
The principle that ties the next several decisions together: one schema per mode, with optional fields included only when explicitly requested. No `null` placeholders, no shape switching.

- **Query mode JSON without `--fields`**: `[{"file": "path", "frontmatter": {...full FM}}]`.
- **Query mode JSON with `--fields a,b`**: `[{"file": "path", "frontmatter": {"a": ..., "b": ...}}]` (frontmatter narrowed to requested fields).
- **Values mode JSON without `--count`**: `[{"value": "..."}, ...]`.
- **Values mode JSON with `--count`**: `[{"value": "...", "count": N}, ...]`.

This is a deliberate reversal from the original "JSON always emits full frontmatter" decision. See ADR 0001 for the reasoning.

### Exit code semantics changed (BREAKING)
See [ADR 0002](../adr/0002-empty-result-is-success.md). v0.3.0 changes the exit code on empty results from `1` to `0`, in both query mode and values mode. Exit `2` still signals usage errors (missing vault, parse errors, incompatible flags). Existing `if ovq ...; then` scripts must switch to output-based detection.

### TSV transport is display-only
TSV with `--fields` is intended for ad-hoc inspection and `awk`/`cut` piping, not for round-trip serialisation. Cell rules:

- **Delimiter**: literal `\t` between columns. No CSV or markdown variants.
- **Array values**: joined with `, ` (literal comma + space). Collisions inside element values are not escaped - if you need round-trip, use `--json`.
- **Tab or newline inside a value**: replaced with a single space to avoid corrupting the TSV grid.
- **Wiki-links**: kept raw (`[[Foo]]`) - consistent with `--json` and with what `Read` sees on disk.
- **Nested object values**: rendered as compact JSON inside the cell (`{"url":"x","width":500}`), tab/newline normalised. Carries enough info for the agent to decide whether to re-run with `--json`.
- **Missing fields**: empty column, not dropped row. Query already decided which rows survive; field presence is orthogonal.

### `--fields` value parsing
- Comma-separated string: `--fields attribution,source,created`.
- Whitespace around each name is trimmed.
- Empty strings (after trim) are skipped (so `--fields a,,b` = `[a, b]`).
- Duplicates are silently deduped, preserving first-occurrence order (so `--fields a,a,b` = `[a, b]`).
- Field name matching against frontmatter is **case-insensitive**, matching the query engine's behaviour (`--fields Status` finds `status:`).
- **Flat names only.** No dot-path for nested access. If a field is itself an object, the whole object is selected; see "nested object values" above for TSV rendering. YAGNI now; revisit if a real nested-frontmatter use case shows up.

### JSON formatting
- **Compact, single-line.** No pretty-print flag. Pipe through `jq` for human-readable formatting. Matches `gh`, `kubectl`, `cargo` defaults.
- **YAML datetimes** in frontmatter render as ISO 8601 strings in JSON (`"2024-12-16"` for date, `"2024-12-16T10:30:00"` for datetime). JSON has no native datetime type, ISO 8601 is the universal carrier.
- **Array fields** remain JSON arrays (no `, ` joining - that's a TSV-only concern).
- **Wiki-links** stay raw (`"author": "[[Steve Jobs]]"`).

### Path display
Same logic everywhere (plain / TSV / JSON): vault-relative when the file lives under `--vault`, absolute fallback otherwise (matches the existing query-mode behaviour for `--stdin` inputs from outside the vault).

## Implementation Steps

### Task 1: Add `src/output.rs` with helpers and TSV formatter

- [x] add `mod output;` in `src/main.rs`
- [x] create `src/output.rs` with `pub fn format_path(path: &Path, vault: &Path) -> String` that produces the existing vault-relative-or-absolute path string. Both formatters use this one helper.
- [x] add a `pub fn parse_fields(spec: &str) -> Vec<String>` free function: split on `,`, trim each, drop empties, dedupe preserving first-occurrence order
- [x] add `pub fn format_tsv(path: &Path, vault: &Path, fm: &YamlValue, fields: &[&str]) -> String` (returns one line without trailing newline)
  - case-insensitive field lookup against frontmatter keys
  - scalar values rendered as-is, with tab/newline normalised to single space
  - YAML sequences joined with `, ` (literal comma + space)
  - YAML mappings rendered as compact JSON inside the cell
  - missing field renders as empty column
  - wiki-links kept raw (no `[[]]` stripping)
- [x] write tests for `parse_fields`: trim, drop empties, dedupe, order preservation, single value, empty input
- [x] write tests for `format_tsv`:
  - all fields present, scalar values
  - missing field renders as empty column
  - case-insensitive lookup: `--fields Status` finds `status:`
  - YAML sequence joined with `, `
  - YAML mapping rendered as compact JSON in the cell
  - tab and newline in a string value normalised to space
  - mixed types (string + number + bool + date)
  - wiki-link string kept raw
- [x] run `cargo test` - all tests must pass before task 2

Note: serde_json dep (listed in Task 2) was pulled forward into Task 1 because TSV mapping cells require it; a minimal `yaml_to_json` helper was also added in this task. Task 2 still owns the date-handling refinement and `format_json_query` / `format_json_values`.

### Task 2: Add JSON formatter (`format_json`)

- [x] add `serde_json = "1"` to `Cargo.toml` dependencies (already done in Task 1; verified present)
- [x] in `src/output.rs`, add `pub fn format_json_query(matches: &[(PathBuf, YamlValue)], vault: &Path, fields: Option<&[&str]>) -> String`
  - builds `Vec<serde_json::Value>` of `{ "file": "<vault-relative path>", "frontmatter": <fm-as-json> }`
  - when `fields` is `Some(list)`, narrow `frontmatter` to only those keys (case-insensitive lookup, same as TSV)
  - serialise with `serde_json::to_string` (compact, no `to_string_pretty`)
- [x] add `pub fn format_json_values(counts: &HashMap<String, usize>, show_count: bool) -> String`
  - builds `[{value}]` if `show_count == false`, `[{value, count}]` if `true`
  - sort the array using the same ordering values mode already uses for plain output (alphabetic without count, count-desc with count) so JSON and plain ordering match
- [x] write a `pub(crate) fn yaml_to_json(v: &YamlValue) -> serde_json::Value` helper: walk YAML, produce equivalent JSON. YAML datetimes render as ISO 8601 strings (serde_yaml parses dates as strings, so this is automatic). Unrepresentable values (tagged scalars, etc) fall back to `null` (already done in Task 1).
- [x] write tests for `format_json_query`:
  - empty matches -> `[]`
  - single match with scalar fields, no `--fields` (full frontmatter)
  - single match with `--fields a,b` narrows to those keys only
  - YAML array stays JSON array (no comma-joining)
  - nested map stays nested JSON object
  - YAML date renders as `"YYYY-MM-DD"` string
  - vault-relative path correct, absolute fallback when path outside vault
- [x] write tests for `format_json_values`:
  - empty counts -> `[]`
  - `[{value}]` without count
  - `[{value, count}]` with count
  - sorting matches plain mode
- [x] run `cargo test` - all tests must pass before task 3

Note: missing requested field in `--fields` narrowing is omitted from the JSON `frontmatter` object (no `null` placeholder), matching ADR 0001's "requested fields included, unrequested omitted" principle.

### Task 3: Wire `--fields` and `--json` flags into the CLI

- [x] add `#[arg(long, help = "Output selected frontmatter fields as TSV (comma-separated field names)")] fields: Option<String>` to `Cli` in `src/main.rs`
- [x] add `#[arg(long, help = "Output as JSON")] json: bool` to `Cli`
- [x] reject incompatible combinations BEFORE running queries:
  - `cli.values.is_some() && cli.fields.is_some()` -> stderr error "--fields cannot combine with --values", exit 2
- [x] in `run_query_mode`, collect the matching `(PathBuf, YamlValue)` pairs into a `Vec` instead of printing during iteration. Plain mode now buffers too - this is a conscious deviation from the previous streaming behaviour, deliberate so all three transport formats share one code path. Document the trade-off in the task PR description.
- [x] dispatch:
  - `cli.json` set: call `output::format_json_query(matches, vault, parsed_fields.as_deref())`, print result
  - `cli.fields` set (no `--json`): for each match, call `output::format_tsv(...)` and print line
  - neither set: keep existing one-path-per-line behavior
- [x] in `run_values_mode`, REMOVE the existing `if counts.is_empty() { return ExitCode::from(1); }` early return at `src/main.rs:78-80`. Empty counts must now flow through to the chosen formatter; plain mode emits nothing (as it does today after the early return), JSON mode emits `[]`.
- [x] in `run_values_mode`, dispatch on `cli.json`: `format_json_values(counts, cli.count)` when set, existing plain output via `format_values` otherwise.
- [x] in `run_query_mode`, REMOVE the final `if found { ExitCode::from(0) } else { ExitCode::from(1) }` branch at `src/main.rs:116-120`. Replace with unconditional `ExitCode::from(0)`. Per ADR 0002, a successful run is exit 0 regardless of result count.
- [x] extract the exit-code decision logic into a small pure helper so the change is unit-testable. Implemented as `fn exit_for_query_run(_matched_count: usize) -> u8` and `fn exit_for_values_run(_count: usize) -> u8` returning `u8` (then wrapped in `ExitCode::from`) because `std::process::ExitCode` does not implement `PartialEq` and has no public extractor, making `u8` the only ergonomic shape for direct assertions.
- [x] write tests for the exit-code helpers:
  - `exit_for_query_run(0)` returns 0
  - `exit_for_query_run(N)` returns 0 for any N
  - `exit_for_values_run(0)` returns 0
  - `exit_for_values_run(N)` returns 0 for any N
- [x] write tests for any CLI-parsing logic outside clap's derive (`parse_fields` covered in Task 1; the `--values + --fields` rejection is a single boolean conjunction inside `main()` - not unit-testable without restructuring, will be exercised via Task 4 smoke test and Task 5 acceptance)
- [x] run `cargo test` - all tests must pass before task 4

### Task 4: Smoke test, lint, docs

- [x] run `cargo build --release` - must succeed with no warnings
- [x] run `cargo clippy --all-targets -- -D warnings` - must be clean (fixed 4 pre-existing lints: `map_or`->`is_none_or`/`is_some_and` in parser.rs and vault.rs, `flatten`->`map_while(Result::ok)` in vault.rs, `&PathBuf`->`&Path` in main.rs)
- [x] smoke test against `~/Obsidian/PK Workspace` (use the real vault for one end-to-end sanity check):
  - `cargo run -- --vault "$HOME/Obsidian/PK Workspace" --fields attribution,source 'categories contains "Quotes"'`
  - `cargo run -- --vault "$HOME/Obsidian/PK Workspace" --json 'categories contains "Quotes"' | head -c 500`
  - verify TSV has 11 lines (one per Quote) and JSON parses with `jq`
- [x] update `README.md`:
  - add `--fields` and `--json` to the Usage section with one example each
  - add a "Output formats" subsection explaining when to use which
- [x] update `CLAUDE.md` architecture section to mention `src/output.rs` and the formatter split
- [x] run `cargo test` final pass

### Task 5: Verify acceptance criteria

- [x] verify all requirements from Overview are implemented
- [x] verify plain mode output is unchanged: `cargo run -- 'status = "active"'` still emits one path per line. Only the exit code on empty result changes (now 0, was 1) - see ADR 0002.
- [x] verify `--fields` works with all value types observed in the vault (scalar string, number, bool, date, wiki-link, array, nested map)
- [x] verify `--json` output is valid JSON parseable by `jq` and `python -m json.tool`
- [x] verify `--json + --fields` narrows `frontmatter` to the requested keys (ADR 0001)
- [x] verify `--values --json` emits the unified `[{value, count?}]` schema in both `--count` and no-`--count` modes
- [x] verify `--values + --fields` exits 2 with the documented error
- [x] verify empty result exit code is 0 in all modes (plain, TSV, JSON, values) per ADR 0002
- [x] run full test suite (`cargo test`) - all green
- [x] run linter (`cargo clippy --all-targets -- -D warnings`) - all green

## Technical Details

- **`src/output.rs` shape** (proposed):
  ```rust
  use serde_yaml::Value as YamlValue;
  use std::path::{Path, PathBuf};
  use std::collections::HashMap;

  pub fn format_path(path: &Path, vault: &Path) -> String { ... }
  pub fn parse_fields(spec: &str) -> Vec<String> { ... }
  pub fn format_tsv(path: &Path, vault: &Path, fm: &YamlValue, fields: &[&str]) -> String { ... }
  pub fn format_json_query(matches: &[(PathBuf, YamlValue)], vault: &Path, fields: Option<&[&str]>) -> String { ... }
  pub fn format_json_values(counts: &HashMap<String, usize>, show_count: bool) -> String { ... }

  pub(crate) fn yaml_to_json(v: &YamlValue) -> serde_json::Value { ... }
  fn render_cell_for_tsv(v: &YamlValue) -> String { ... }
  fn lookup_field_ci<'a>(fm: &'a YamlValue, name: &str) -> Option<&'a YamlValue> { ... }
  ```
- **CLI changes** (`src/main.rs`):
  ```rust
  #[arg(long, help = "Output selected frontmatter fields as TSV (comma-separated field names)")]
  fields: Option<String>,

  #[arg(long, help = "Output as JSON")]
  json: bool,
  ```
- **Refactor inside `run_query_mode`**: collect matches into `Vec<(PathBuf, YamlValue)>` first, then dispatch on the output flags. Slight memory cost (holds all matches in RAM) but the vault has ~1.4k files - trivial.
- **`fields` parsing**: `Vec<String>` from `cli.fields.unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()`. Empty string after split = ignored.

## Post-Completion

*Items requiring manual intervention or external systems - no checkboxes, informational only*

**Release / distribution:**
- After merge, bump `Cargo.toml` to v0.3.0 and tag `v0.3.0`. The existing `.github/workflows/release.yml` should pick up the tag and ship the Homebrew formula update via `pkarpovich/apps`.
- Run `brew upgrade pkarpovich/apps/ovq` on the dev machine to confirm the new binary works locally.

**Downstream skill updates:**
- The `ovq` skill in `~/Projects/environment/dotfiles/claude/skills/ovq/SKILL.md` should be updated to document `--fields` and `--json` in the Discovery section and Quick Start. Do this in the dotfiles repo, not here.
- Re-run `skill-audit` on the updated ovq skill to confirm description stays under 1024 chars.

**Manual verification (optional):**
- Compare `--fields` output for a Movies query against `cargo run --release` performance on the ~1.4k-file vault. Expectation: still sub-second.
