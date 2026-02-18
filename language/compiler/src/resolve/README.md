# Resolve

Resolve connects modules into a graph and figures out what cross-module references point to for a specific profile.
Resolve is the first phase that treats the program as a system rather than isolated trees.

Builtin alias metadata is defined by builtin lib metadata and consumed during import resolution.
Protocol aliases such as `node:` and `bun:` are handled in compiler-side import resolution and then flow through normal module target handling.
