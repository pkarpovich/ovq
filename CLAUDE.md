# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build              # Build the project
cargo build --release    # Build optimized binary
cargo test               # Run all tests
cargo test parser        # Run tests matching "parser"
cargo run -- --vault /path/to/vault 'status = "active"'  # Run with args
```

## Architecture

**ovq** is a CLI tool for querying Obsidian vault markdown files by their YAML frontmatter properties, using Dataview-style WHERE syntax.

### Core Flow
1. `main.rs` - CLI entry point using clap, dispatches to query mode or values mode, then routes matches through an output formatter
2. `vault.rs` - Collects markdown files (respects `.gitignore` and `.obsidianignore`)
3. `frontmatter.rs` - Extracts and parses YAML frontmatter from markdown files
4. `query/` - Query parsing and evaluation:
   - `ast.rs` - Expression types: `Compare`, `Contains`, `And`, `Or` with operators and value types
   - `parser.rs` - Recursive descent parser for Dataview WHERE syntax
   - `eval.rs` - Evaluates parsed expressions against frontmatter YAML
5. `values.rs` - Aggregates unique property values with optional counts
6. `output.rs` - Transport formatters shared by both modes:
   - `format_path` / default plain output (one path per line)
   - `parse_fields` + `format_tsv` for `--fields` (TSV with case-insensitive lookup, array join, mapping-as-JSON cells)
   - `format_json_query` and `format_json_values` for `--json` (compact JSON, unified schema per ADR 0001)
   - `yaml_to_json` helper bridging `serde_yaml::Value` to `serde_json::Value`

### Query Syntax
- Comparisons: `field = "value"`, `field > 5`, `field >= 2024-01-01`
- Contains: `tags contains "project"` (works with arrays)
- Boolean: `expr AND expr`, `expr OR expr`, parentheses for grouping
- Values: strings (`"quoted"`), numbers, booleans (`true`/`false`), dates (`YYYY-MM-DD`)

### Matching Behavior
- Field names are case-insensitive
- String comparisons are case-insensitive
- Obsidian wiki-links (`[[Link]]`) are automatically stripped for comparison
