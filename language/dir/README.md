# dir

Main Destack IR (DIR) definitions and program model.
DIR is the semantic intermediate representation that we do (almost) all of our semantic analysis on with all the classic stuff about symbols, scopes, and types.

## Overview

DIR is mapped from AST during binding (with a pretty close 1:1 correspondence), and then we do a _lot_ of in-place processing and transformation before ultimately delegating to Lower (MIR) or straight codegen (JS/TS).

```text
AST (syntax)  →  DIR (semantics)  →  MIR (machine)
    │                  │
  parser            compiler
```

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_dir
just language/test-specification
just language/test-emit

# clean gate
just language/quick

# exhaustive gate
just language/full
```
