# Language

The Destack language toolchain, written in Rust (for now).
See [DESIGN.md](DESIGN.md) for language design and [SPECIFICATION.md](SPECIFICATION.md) for syntax/semantics.

## Compilation

Destack source (`.ds`, `.ts`, `.js`) compiles through several intermediate representations:

```
Source ───► AST ───► DIR ───► JS/TS
             │        │
             │        └───► MIR ───► WASM/Native (future)
             │
        (syntax)    (typed, semantic)    (low-level)
```

Each representation serves a different purpose in the compilation pipeline:

| Representation | Description |
|----------------|-------------|
| AST | Abstract Syntax Tree - untyped syntax, close to source |
| DIR | Data-level IR (Destack IR) - typed semantic representation with symbols and scopes |
| MIR | Machine-level IR - low-level representation for native codegen |

For JS/TS targets, compilation may skip the middle-end and go directly from DIR to output (no MIR needed).
See [compiler/README.md](compiler/README.md) for the full pipeline.

## Crates

The language toolchain is split into several crates, each handling a specific part of the pipeline:

| Crate | Description | Link |
|-------|-------------|------|
| `ast` | Destack AST definition | [ast/](ast/) |
| `dir` | Destack DIR and program definition | [dir/](dir/) |
| `parser` | Destack lexer and parser (source → AST) | [parser/](parser/) |
| `compiler` | Destack compiler (AST → DIR → MIR) | [compiler/README](compiler/README.md) |
| `resolver` | Destack module resolution | [resolver/](resolver/) |
| `formatter` | Destack formatter (for `.ds`) | [formatter/](formatter/) |
| `linter` | Destack linter | [linter/](linter/) |
| `source` | Destack source handling | [source/](source/) |
| `unicode` | Unicode property tables and utilities | [unicode/](unicode/) |
| `workspace` | Destack multi-program workspaces | [workspace/](workspace/) |

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
