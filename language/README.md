# Language

The Destack language toolchain, written in Rust.
See [DESIGN.md](DESIGN.md) for language design and semantics.

The Destack compiler takes source files from a supported language (`.ds`, `.ts`/`.tsx`, `.js`/`.jsx`) and transforms them into final output via several intermediate representations (AST → DIR → MIR). See [compiler/README.md](compiler/README.md) for the full pipeline.

## Projects

The language toolchain is split into several Rust crates, each handling a specific part of the pipeline.

| Project | Status | Summary |
|---------|--------|---------|
| [`compiler`](compiler/README.md) | Alpha | End-to-end compiler from AST through DIR to MIR |
| [`formatter`](formatter/README.md) | Alpha | Canonical source formatter |
| [`linter`](linter/README.md) | Alpha | Linter rules and linting interface |
| [`lsp`](lsp/README.md) | Alpha | Language Server Protocol service implementation |
| [`parser`](parser/README.md) | Alpha | Lexer and parser for `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx` |
| [`query`](query/README.md) | Alpha | Semantic tooling queries and presentation formatting |
| [`resolver`](resolver/README.md) | Alpha | JS and TS style module and package resolution |
| [`runtime`](runtime/README.md) | Experimental | Native runtime, platform bindings, and host integration |
| [`workspace`](workspace/README.md) | Alpha | Stateful multi-program workspaces |

## Commands

Run these commands from the repository root.

```sh
just language/install
just language/format
just language/format-check
just language/check
just language/build
just language/test
just language/quick
just language/full
just language/bench
just language/fuzz 60
```
