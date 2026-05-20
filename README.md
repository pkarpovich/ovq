# ovq

Query Obsidian vault files by frontmatter properties using Dataview-style syntax.

## Why ovq?

**Built for Claude Code** to search your Obsidian vault semantically by frontmatter metadata.

### The Problem with grep

Obsidian notes use YAML frontmatter for structured metadata:

```yaml
---
status: active
priority: 3
tags: [project, work]
created: 2024-06-15
---
```

Using `grep` to find notes fails in several ways:

| Task | grep approach | Problem |
|------|--------------|---------|
| Find `status: active` | `grep "status: active"` | Matches "status: active" in body text too |
| Find `priority > 2` | Not possible | grep can't do numeric comparisons |
| Find notes with tag "project" | `grep "project"` | Matches anywhere, not just in tags array |
| Find `created >= 2024-01-01` | Not possible | grep can't compare dates |
| Find notes with `due` property | `grep "due:"` | Matches "due:" anywhere in file |
| Case-insensitive field match | Complex regex | `Status:` vs `status:` requires extra work |

### The ovq solution

```bash
ovq 'status = "active"'                    # Only matches frontmatter field
ovq 'priority > 2'                         # Numeric comparison
ovq 'tags contains "project"'              # Array-aware search
ovq 'created >= 2024-01-01'                # Date comparison
ovq 'status = "active" AND priority > 2'   # Combine conditions
ovq 'due'                                  # Property exists and is truthy
ovq 'due != null'                          # Property exists (any value)
```

## Claude Code Integration

Set your vault path once:

```bash
export OVQ_VAULT=/path/to/your/vault
```

Now Claude Code can query your knowledge base:

```bash
# Find all active projects
ovq 'status = "active" AND tags contains "project"'

# Find recent meeting notes
ovq 'type = "meeting" AND created >= 2024-01-01'

# Find high-priority todos
ovq 'priority >= 4 AND status != "done"'
```

Output is a list of matching file paths, ready for Claude to read and use.

## Installation

### Homebrew (macOS)

```bash
brew install pkarpovich/apps/ovq
```

### From source

```bash
cargo install --path .
```

## Usage

```bash
# Query files
ovq 'status = "active"'
ovq --vault /path/to/vault 'tags contains "work"'

# List unique values for a property
ovq --values status
ovq --values tags --count

# Pipe file paths
find ~/vault -name "*.md" | ovq --stdin 'priority > 3'

# Pull frontmatter fields alongside each match (TSV: path<TAB>field1<TAB>field2)
ovq --fields attribution,source 'categories contains "Quotes"'

# Emit JSON for jq / scripting (works in query and values modes)
ovq --json 'status = "active"'
ovq --json --fields title,priority 'tags contains "work"'
ovq --values status --json --count
```

## Output formats

Default output is one matching file path per line. Two flags change the transport:

- `--fields a,b,c` writes a TSV row per match: `path<TAB>a<TAB>b<TAB>c`. Pipe through `awk`, `cut`, `column -t`. Array fields are joined with `, `; nested objects render as compact JSON in the cell; tabs and newlines inside values are flattened to a single space. Use for ad-hoc inspection.
- `--json` emits a compact JSON array. In query mode each entry is `{"file": ..., "frontmatter": {...}}` (narrowed to requested keys when combined with `--fields`). In values mode each entry is `{"value": ...}` or `{"value": ..., "count": N}` when `--count` is set. Use when you need to `jq` the result or feed it to another tool.

`--values` and `--fields` are mutually exclusive (exit 2). Empty results exit 0 in every mode; exit 2 still signals usage errors.

## Query Syntax

### Operators

`=`, `!=`, `>`, `<`, `>=`, `<=`

### Contains

```bash
ovq 'tags contains "project"'   # Array membership
ovq 'title contains "meeting"'  # Substring match
```

### Existence Checks

```bash
ovq 'due'                  # Property exists and is truthy
ovq '!due'                 # Property missing or falsy
ovq 'due != null'          # Property exists (even if empty)
ovq 'due = null'           # Property is missing
```

### Boolean Logic

```bash
ovq 'status = "active" AND priority > 2'
ovq 'status = "done" OR status = "archived"'
ovq '(type = "note" OR type = "doc") AND status = "published"'
```

### Value Types

- Strings: `"quoted"`
- Numbers: `42`, `3.14`
- Booleans: `true`, `false`
- Dates: `2024-01-15`
- Null: `null`

## Matching Behavior

- Field names: case-insensitive
- String values: case-insensitive
- Obsidian links: `[[Link]]` normalized to `Link`

## File Discovery

- Scans `.md` files recursively
- Respects `.gitignore` and `.obsidianignore`

## License

MIT
