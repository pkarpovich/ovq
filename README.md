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
| Case-insensitive field match | Complex regex | `Status:` vs `status:` requires extra work |

### The ovq solution

```bash
ovq 'status = "active"'                    # Only matches frontmatter field
ovq 'priority > 2'                         # Numeric comparison
ovq 'tags contains "project"'              # Array-aware search
ovq 'created >= 2024-01-01'                # Date comparison
ovq 'status = "active" AND priority > 2'   # Combine conditions
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
```

## Query Syntax

### Operators

`=`, `!=`, `>`, `<`, `>=`, `<=`

### Contains

```bash
ovq 'tags contains "project"'   # Array membership
ovq 'title contains "meeting"'  # Substring match
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

## Matching Behavior

- Field names: case-insensitive
- String values: case-insensitive
- Obsidian links: `[[Link]]` normalized to `Link`

## File Discovery

- Scans `.md` files recursively
- Respects `.gitignore` and `.obsidianignore`

## License

MIT
