# Language

The Destack language toolchain, written in Rust (for now).
See [DESIGN.md](DESIGN.md) for design philosophy and [SPECIFICATION.md](SPECIFICATION.md) for precise syntax and semantics.

The Destack compiler takes source files from a supported language (`.ds`, `.ts`/`.tsx`, `.js`/`.jsx`) and compiles them to final output via several intermediate representations (AST → DIR → MIR). See [compiler/README.md](compiler/README.md) for the full pipeline.

## Crates

The language toolchain is split into several crates, each handling a specific part of the pipeline:

| Crate | Description | Link |
|-------|-------------|------|
| `base` | Shared utilities (ids, interning, small helpers) | [base/](base/) |
| `source` | Source files, spans, diagnostics | [source/](source/) |
| `unicode` | Unicode property tables and utilities | [unicode/](unicode/) |
| `ast` | AST definition | [ast/](ast/) |
| `parser` | Lexer and parser (`.(js|jsx|ts|tsx|ds)` → AST) | [parser/](parser/) |
| `dir` | DIR and program definition | [dir/](dir/) |
| `mir` | MIR and formatting utilities | [mir/README.md](mir/README.md) |
| `vm` | MIR interpreter for comptime, debug, and deopt | [vm/README.md](vm/README.md) |
| `codegen` | Code generation backends (JS, Cranelift) | [codegen/README.md](codegen/README.md) |
| `codegen/lib` | Shared codegen helpers | [codegen/lib/](codegen/lib/) |
| `compiler` | End-to-end compiler (AST → DIR → MIR) | [compiler/README.md](compiler/README.md) |
| `compiler/macros` | Compiler procedural macros | [compiler/macros/](compiler/macros/) |
| `runtime` | Native and WASM runtime support | [runtime/README.md](runtime/README.md) |
| `builtin` | Builtin libraries and definitions | [builtin/README.md](builtin/README.md) |
| `resolver` | JS/TS-style module resolution | [resolver/](resolver/) |
| `formatter` | Source formatter (for `.ds` only) | [formatter/](formatter/) |
| `fir` | Formatter IR used by the formatter and emitters | [fir/README.md](fir/README.md) |
| `linter` | Linter rules and interface | [linter/](linter/) |
| `linter/macros` | Linter procedural macros | [linter/macros/](linter/macros/) |
| `workspace` | Stateful, multi-program workspaces | [workspace/](workspace/) |
| `test` | Integration tests and fixtures | [test/README.md](test/README.md) |

## Commands

Common development commands:

```sh
just language/check   # cargo clippy --release
just language/build   # cargo build --release
just language/test    # cargo test
just language/format  # cargo fmt
just language/bench   # cargo bench
```
