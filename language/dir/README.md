# DIR

Main Destack IR (DIR) definitions and program model with DIR as the semantic intermediate representation that we do (almost) all of our semantic analysis on, with all the classic stuff like symbols, scopes, and types.

## Overview

The parser builds DIR directly from source: there is no separate AST or CST, DIR is both.
The compiler then does a _lot_ of in-place processing and transformation on it before ultimately delegating to Lower (MIR) or straight codegen (JS/TS).

```text
source text  →  DIR (syntax + semantics)  →  MIR (machine)
          parser                    compiler
```

The basic shape of the DIR is one big tree plus plenty of side tables: analysis results go into per-module tables keyed by node, and the nodes themselves stay as parsed.

```text
dir
├── source   // tokens, identifiers, spans, comments
├── tree     // the semantic tree: declarations, expressions, patterns, ...
├── symbol   // symbols, scopes, modules, exports, language items
├── table    // per-module side tables of analysis facts
└── type     // the type representation and its relations
```

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_dir
just language/test-specification

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
