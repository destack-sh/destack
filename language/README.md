# Language

The Destack language toolchain, written in Rust.
Handles parsing, compilation, formatting, resolution, and editor tooling for `.ds` and `.ts` files.

## Crates

| Crate | Description |
|-------|-------------|
| `ast` | Destack AST definition |
| `dir` | Destack DIR and program definition |
| `fir` | Destack formatting IR |
| `parser` | Detsack lexer and parser (text->tokens->AST) |
| `compiler` | Destack compiler (AST->DIR->MIR) |
| `resolver` | Destack module resolution |
| `formatter` | Destack formatter (for `.ds`) |
| `linter` | Destack linter |
| `source` | Destack source handling |
| `unicode` | Unicode property tables and utilities |
| `workspace` | Destack multi-program workspaces |

### JavaScript

| Crate | Description |
|-------|-------------|
| `javascript/ast` | JavaScript AST |
| `javascript/transpiler` | Destack to JavaScript transpiler (DIR->JS/TS) |

## Commands

```sh
just language/check   # cargo check
just language/build   # cargo build --release
just language/test    # cargo test
just language/fmt     # cargo fmt
just language/lint    # cargo clippy
```
