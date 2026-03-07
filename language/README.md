# Language

The Destack language toolchain, written in Rust (for now).
See [DESIGN.md](DESIGN.md) for design philosophy and [SPECIFICATION.md](SPECIFICATION.md) for (more) detailed semantics.
See [COMPATIBILITY.md](COMPATIBILITY.md) for the language and target compatibility matrix.

The Destack compiler takes source files from a supported language (`.ds`, `.ts`/`.tsx`, `.js`/`.jsx`) and transforms them into final output via several intermediate representations (AST → DIR → MIR). See [compiler/README.md](compiler/README.md) for the full pipeline.

## Crates

The language toolchain is split into several crates, each handling a specific part of the pipeline:

| Crate | Description | Link |
|-------|-------------|------|
| `ast` | CST-style AST definition | [ast/README.md](ast/README.md) |
| `base` | Shared utilities (ids, interning, small helpers) | [base/README.md](base/README.md) |
| `builtin` | Builtin libraries and definitions | [builtin/README.md](builtin/README.md) |
| `codegen` | Code generation backends (JS, Cranelift) | [codegen/README.md](codegen/README.md) |
| `compiler` | End-to-end compiler (AST → DIR → MIR) | [compiler/README.md](compiler/README.md) |
| `dir` | High-level DIR and program definition | [dir/README.md](dir/README.md) |
| `fir` | Formatter IR used by all formatters | [fir/README.md](fir/README.md) |
| `formatter` | Source formatter (for `.ds` only) | [formatter/README.md](formatter/README.md) |
| `heap` | Heap values and runtime memory structures | [heap/README.md](heap/README.md) |
| `json` | JSON and JSONC AST, parser, and formatter | [json/README.md](json/README.md) |
| `linter` | Linter rules and interface | [linter/README.md](linter/README.md) |
| `mir` | Machine-level IR | [mir/README.md](mir/README.md) |
| `parser` | Lexer and parser (`.(js,jsx,ts,tsx,ds)` → AST) | [parser/README.md](parser/README.md) |
| `resolver` | JS/TS-style module resolution | [resolver/README.md](resolver/README.md) |
| `runtime` | Native and WASM runtime support | [runtime/README.md](runtime/README.md) |
| `service` | Workspace-rooted language tooling orchestration | [service/README.md](service/README.md) |
| `source` | Source files, spans, diagnostics | [source/README.md](source/README.md) |
| `test` | Integration tests and fixtures | [test/README.md](test/README.md) |
| `unicode` | Unicode property tables and utilities | [unicode/README.md](unicode/README.md) |
| `vm` | MIR interpreter for comptime, debug, and deopt | [vm/README.md](vm/README.md) |
| `workspace` | Stateful, multi-program workspaces | [workspace/README.md](workspace/README.md) |

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
