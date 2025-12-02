# Language

The Destack language toolchain, written in Rust (for now).
See [DESIGN.md](DESIGN.md) for design philosophy and [SPECIFICATION.md](SPECIFICATION.md) for precise syntax and semantics.

The Destack compiler takes source files from a supported language (`.ds`, `.ts`/`.tsx`, `.js`/`.jsx`) and compiles them to some final output via several intermediate representations (AST->DIR->MIR). See [compiler/README.md](compiler/README.md) for the full pipeline.

## Crates

The language toolchain is split into several crates, each handling a specific part of the pipeline:

| Crate | Description | Link |
|-------|-------------|------|
| `ast` | AST definition | [ast/](ast/) |
| `dir` | DIR and program definition | [dir/](dir/) |
| `parser` | Lexer and parser (`.(js|jsx|ts|tsx|ds)` → AST) | [parser/](parser/) |
| `compiler` | End-to-end compiler (AST → DIR → MIR) | [compiler/README](compiler/README.md) |
| `resolver` | JS/TS-style module resolution | [resolver/](resolver/) |
| `formatter` | Source formatter (for `.ds` only) | [formatter/](formatter/) |
| `linter` | Linter rules and linter interface| [linter/](linter/) |
| `source` | Source, files, diagnostics | [source/](source/) |
| `unicode` | Unicode property tables and utilities | [unicode/](unicode/) |
| `workspace` | Stateful, multi-program workspaces | [workspace/](workspace/) |

### JavaScript

These crates handle JavaScript/TypeScript transpilation:

| Crate | Description | Link |
|-------|-------------|------|
| `javascript/ast` | JavaScript AST | [javascript/ast/](javascript/ast/) |
| `javascript/transpiler` | Destack to JavaScript / TypeScript transpiler (DIR/MIR → JS/TS) | [javascript/transpiler/](javascript/transpiler/) |

## Commands

Common development commands:

```sh
just language/check   # cargo check
just language/build   # cargo build --release
just language/test    # cargo test
just language/fmt     # cargo fmt
just language/lint    # cargo clippy
```
