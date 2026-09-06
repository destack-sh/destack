---
title: Decorators
description: decorators on expressions
---

# Decorators

- JS/TS already sorta kinda has decorators, sometimes
- extended placement
- decorators on expressions
- newtypes as decorators
- incl. union newtypes

- queryable
- `@if` static gating
- `@if(staticTerm)` removes imports, declarations, members, statements, cases, arguments, fields, and similar contributions before checking and output
- `@if` is resolved before inference

- `@allow`, `@warn`, `@deny`, `@forbid`, and `@expect` tune diagnostics lexically; conditional forms may use static metadata and carry a reason
- `@unsafe` marks an unsafe operation, while `@safe` exposes a checked API whose implementation contains unsafe operations
- `@derive` synthesizes compiler-known interfaces such as `Copy`, `SharedSafe`, `Clone`, `Default`, comparison, formatting, hashing, and serialization capabilities
