# Destack Compiler

The Destack compiler takes JavaScript, TypeScript and Destack source files (`.(ds|ts|tsx|js|jsx)`) and by the power of magic and the art of computer science transforms them into executable artifacts like (`.(js|ts|wasm|o)`) with a (mostly) classic multi-phase compilation pipeline.
Most of the compilation logic lives in `language/compiler`, with a few externalized crates for better organization:
 - **Import** (I) uses `language/parser` for parsing code modules (but not data modules)
 - **Resolve** (R) uses `language/resolver` for module path resolution (but not symbol resolution)
 - **Generate** (G) uses `language/codegen` for most of the actual codegen (incl. vendored Cranelift)
 - **Lint** (L) uses `language/linter` for the main linting logic

## Pipeline

Like most compilers, the Destack compiler has three main regions:
 1. Front-end (source `.(ds|ts|tsx|js|jsx)` → fully typed, elaborated canonical DIR per profile)
 2. Middle-end (execute comptime, then lower to target-specific MIR)
 3. Back-end (DIR/MIR → emitted artifacts depending on target).

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                                FRONT-END                                    │
│                                                                             │
│  Stages:  Import ───► Resolve ───► Analyze ───► Elaborate                   │
│  Output: base DIR   resolved DIR  typed DIR   canonical DIR                 │
│                                                                             │
│                    (base DIR shared, canonical DIR per profile)             │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                        (one canonical DIR per profile)
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                MIDDLE-END                                   │
│                                                                             │
│  Stages:  Execute ───► Lower ───► Optimize                                  │
│  Output: patched DIR     MIR    optimized MIR                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                         (one MIR per target)
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                 BACK-END                                    │
│                                                                             │
│  Stages:  Generate ───► Link ───► Emit                                      │
│  Output:  artifacts    linked    files (.js, .wasm, .o)                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Representations

The compiler uses three main intermediate representations from raw syntax form (AST) to canonical analyzed form (DIR) to canonical machine form (MIR).

| Representation | Full Name | Description |
|----------------|-----------|-------------|
| AST | Abstract Syntax Tree | Untyped syntax tree, close to source text (~=CST) |
| DIR | Destack IR | Semantic IR with symbols, scopes, and types (base, canonical, patched) |
| MIR | Machine IR | Monomorphic, target-aware IR for comptime execution, optimization, and native codegen |

## Phases

The compiler is divided into ~10 phases from raw text to final output, some of which may be skipped, and most of which can run in parallel across different modules (but never per module).
Each phase is identified by a single letter for tracing and diagnostics, and just because that seems kind of nice.

Note that linting is a separate "phase" that conceptually runs alongside the pipeline; this is very convenient for running zero-copy checks in-process on the same rich IRs (and in parallel, too).

### Front-End

The "front-end" transforms source text into typed, elaborated ("canonical") DIR.
Import is **profile-independent**, producing shared base DIR (parse, bind symbols/scopes, desugar).
Resolve, Analyze, and Elaborate are **per-profile**, producing canonical DIR for each profile.

| Phase | Letter | Input | Output | Profile | Description |
|-------|--------|-------|--------|---------|-------------|
| Import | `I` | Text | base DIR | — | Parse source into AST; create DIR with symbols and scopes; desugar syntactic forms (`+=`, `++`, etc.) |
| Resolve | `R` | base DIR | DIR | per-profile | Resolve symbol references (lexical binding, library resolution) |
| Analyze | `A` | DIR | DIR | per-profile | Elaborate type declarations, infer value types, resolve overloads, validate semantics, record instances |
| Elaborate | `E` | DIR | canonical DIR | per-profile | Canonicalize DIR: patterns→decision trees, tree literals→calls, and update type metadata for synthesized nodes. |

### Middle-End

The "middle-end" performs comptime execution, lowers DIR to MIR, and runs verification and optimization.

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Execute | `X` | DIR | DIR | Execute comptime code via internal MIR and patch DIR |
| Lower | `M` | DIR | MIR | Lower patched DIR to MIR (monomorphization, layouts, RTTI as needed) |
| Optimize | `O` | MIR | MIR | Verify MIR then run optimization passes |

### Back-End

The "back-end" generates target artifacts from DIR (for JS/TS) or MIR (for native/WASM).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Generate | `G` | DIR/MIR | artifacts | Generate target code (JS/TS from DIR, native from MIR) |
| Link | `K` | artifacts | linked | Link artifacts into final output |
| Emit | `W` | linked | files | Write linked output to disk |

## Profiles, Platforms and Targets

A **profile** is a semantic configuration, a "comptime world" that determines which symbols exist, how types resolve, and what the global constants (like `import.meta`) are.
A **target** is a build output, with specific settings for code generation, optimization, and output paths.
One profile may map to multiple targets, or put differently, multiple targets may share the same profile for the compiler front-end.

**Profile identity** is ~everything that might affect the middle-end by:
- `output`: OutputFormat (js, ts, wasm, native)
- `runtime`: Runtime (browser, node, deno, bun, wasm-js, wasm-wasi, native-hosted, native-freestanding, native-embedded)
- `platform`: Platform (web, windows, macos, linux, ios, android, wasi, bare-metal, universal)
- `lib`: Normalized library set (e.g., `["esnext", "dom"]`)
- `debug`: Debug flag (affects `import.meta.debug`)
- `env`: Comptime environment snapshot (for `import.meta.env`)
- `flags`: Semantic restriction flags (`no_any`, `no_managed`, `no_exceptions`, etc.)

The compiler also maintains a shared comptime target used by Execute to evaluate static code; it is not tied to any specific output target. 
We need to run comptime on *something*, though how exactly that *should* work is still up for debate.

## Builtins

The compiler loads language builtins from `language/builtin/` as needed based on profile/target configuration.
See [builtin/README.md](../builtin/README.md) for the full structure, basically, it's a bunch of `.d.ts`/`.d.ds` libs plus our own `.ds` stuff.

## Structure

The compiler is organized into modules (roughly) corresponding to each phase, plus some additional administrative modules (like `compile/` and `unbind/` and `tests/`).

| Path | Description | Source |
|------|-------------|--------|
| `compile/` | Compiler orchestration, task queue, and worker threads | [src/compile/](src/compile/) |
| `import/` | Import, parse, bind, and desugar source into base DIR | [src/import/](src/import/) |
| `resolve/` | Resolve symbol references | [src/resolve/](src/resolve/) |
| `analyze/` | Type inference, checking, and overload resolution | [src/analyze/](src/analyze/) |
| `elaborate/` | Post-analysis transforms (patterns, trees, etc.) | [src/elaborate/](src/elaborate/) |
| `execute/` | Execute comptime code and patch DIR | [src/execute/](src/execute/) |
| `lower/` | Lower DIR to MIR | [src/lower/](src/lower/) |
| `optimize/` | Verify and optimize MIR | [src/optimize/](src/optimize/) |
| `generate/` | Generate artifacts from DIR/MIR | [src/generate/](src/generate/) |
| `link/` | Link artifacts | [src/link/](src/link/) |
| `emit/` | Write output files to disk | [src/emit/](src/emit/) |
| `lint/` | Lint rules and diagnostics | [src/lint/](src/lint/) |
| `unbind/` | DIR → AST utilities | [src/unbind/](src/unbind/) |
| `tests/` | Compiler test scaffolding | [src/tests/](src/tests/) |

## Tasks

Tasks are identified by phase letter and sub-code (e.g., `TI001` for Import task 1).
Tasks track file, module, and artifact versions for change detection and dependency tracking.
See `compile/task.rs` for task definitions and `compile/queue.rs` for the task queue.

The compiler uses a parallel task system for concurrent compilation with per-module locking.
Each phase defines _tasks_ that can yield on dependencies and are resumed by the compiler loop once satisfied.
Task dependencies are recorded through `require_*` calls and resolved by the task queue.

## Caching

Incremental compilation reuses phase outputs keyed by some stable combination of file, module, profile, and target versions.
Edits bump `FileVersion` and propagate to `ModuleVersion`.
The compiler, daemon, and LSP share a single canonical cache format for all reusable artifacts.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_compiler
cargo test -p destack_test --test smoke -- --compiler
just language/test-specification
just language/test-query

# clean gate
just language/quick

# exhaustive gate
just language/full
```
