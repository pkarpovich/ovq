# Empty result is success (exit 0), not "nothing found" (exit 1)

Through v0.2.0 ovq followed grep's exit convention: exit 0 if at least one match, exit 1 if zero matches, exit 2 for argument or vault errors. From v0.3.0 onwards, any invocation that runs the query (or values aggregation) to completion exits 0, regardless of how many results came back. Exit 2 still signals usage errors (missing vault, unparseable query, incompatible flag combinations).

The shift is driven by ovq's actual primary user: an LLM agent calling the tool programmatically and reading either plain text or JSON output. For that consumer, "the query ran and returned an empty result" and "the query ran and returned 100 results" are the same outcome — both are success, both are easy to handle by checking the output. Conflating "zero matches" with "the tool failed" through a non-zero exit forced agents to special-case empty results and made shell pipelines like `ovq ... | jq length` require manual exit-code stripping. Aligning with `jq`, `kubectl`, and most modern CLIs that return structured data avoids that ceremony.

This is a breaking change for any caller that uses `if ovq ...; then ...` to mean "did we find something". Such callers must switch to checking the output (`test -n "$(ovq ...)"` or piping through `wc -l`). The README and CHANGELOG for v0.3.0 must call this out so existing scripts can adapt.
