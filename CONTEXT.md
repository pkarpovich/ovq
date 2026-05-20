# ovq

A CLI that answers "which notes in this Obsidian vault satisfy this question about their frontmatter?", expressed in a Dataview-style WHERE syntax. Designed to be piped into other tools - the model that consumes its output is the first-class user.

## Language

**Vault**:
The folder of Markdown files being queried. Identified by `--vault` or `OVQ_VAULT`; treated as the root for path display.

**Frontmatter**:
The YAML block at the top of a Markdown file between `---` fences. Source of truth for all properties ovq reads.

**Property**:
A single key inside the frontmatter (e.g. `status`, `created`, `categories`). Field names are matched case-insensitively.
_Avoid_: "field", "metadata key" - those are general programming terms; "property" is the Obsidian-native name.

**Query**:
A Dataview-style WHERE expression that filters the vault to a subset of files (e.g. `status = "active" AND priority > 2`).

**Mode**:
Which kind of work ovq is doing on this invocation. One of:
- **Query mode**: filter the vault by a query, emit one match per file.
- **Values mode**: aggregate the distinct values of a single property across the vault (driven by `--values`).
Modes are mutually exclusive at the CLI level.

**Transport format**:
How the chosen mode's output is serialised. Orthogonal to mode. One of:
- **Plain**: human-readable text - one path per line in query mode, one value (with optional count) per line in values mode. Default.
- **TSV with fields**: query mode only. One row per match: `path\tfield1\tfield2\t...`. Selected by `--fields`.
- **JSON**: structured array of objects. Applies to both modes. Selected by `--json`.

## Relationships

- A **Query** filters a **Vault** into a set of matching files.
- Every file carries **Frontmatter** which is a map of **Properties**.
- A **Mode** decides what to compute; a **Transport format** decides how to print it. The two are independent axes.
- **TSV with fields** is a transport format that only makes sense in **Query mode**, because it is structured per-file, not per-value.

## Example dialogue

> **Dev:** "Should `--json` work in `--values` mode too?"
> **Pavel:** "Yes - JSON is a transport format. Anything that prints structured data can be wrapped in JSON. The mode picks the shape, the format picks the wire."

## Flagged ambiguities

- "Output" was used to mean both the rendered text and the data structure behind it. Resolved: the data structure is the **Mode** output, the rendering is the **Transport format**.
