# Destack Compiler

The Destack compiler takes JavaScript, TypeScript and Destack sources (`.(ds|ts|tsx|js|jsx)`) and by the power of magic and the art of computer science transforms them into executable artifacts like (`.(js|ts|wasm|o)`) with a modern-ish multi-phase compilation pipeline.

## Pipeline

Like most compilers, the Destack compiler has three main regions:
 1. Front-end (source `.(ds|ts|tsx|js|jsx)` → typed, elaborated canonical DIR per profile)
 2. Middle-end (execute comptime, then lower to target-specific MIR)
 3. Back-end (DIR/MIR → emitted artifacts depending on target).

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                FRONT-END                                    │
│                                                                             │
│  Stages:  Import ──► Bind ──► Resolve ──► Analyze ──► Elaborate              │
│  Output:    AST    base DIR   Symbols      Types   Canonical DIR            │
│                                                                             │
│                (base DIR shared, canonical DIR per profile)                │
└─────────────────────────────────────────────────────────────────────────────┘
                         │ (one canonical DIR per profile)
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                MIDDLE-END                                   │
│                                                                             │
│  Stages:  Execute (comptime MIR) ──► Lower ──► Verify ──► Optimize           │
│  Output:     Patched DIR          MIR      CFG   Canonical MIR              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                       │ (one canonical DIR/MIR per target)
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                 BACK-END                                    │
│                                                                             │
│  Stages:  Generate ──► Link ──► Emit                                         │
│  Output:  Artifacts  Linked  Files                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Phases

Each phase transforms or enriches the `Program`.
Phases are identified by a single letter for tracing and diagnostics.
Linting is a separate phase that runs on DIR alongside the pipeline.

### Front-End

The front-end transforms source text into typed, elaborated ("canonical") DIR.
Import and Bind are **profile-independent**, producing shared base DIR.
Resolve, Analyze, and Elaborate are **per-profile**, producing canonical DIR for each profile.

| Phase | Letter | Input | Output | Profile | Description |
|-------|--------|-------|--------|---------|-------------|
| Import | `I` | Text | AST | — | Parse source into abstract syntax tree |
| Bind | `B` | AST | base DIR | — | Create DIR with symbols and scopes; desugar syntactic forms (`+=`, `++`, etc.) |
| Resolve | `R` | base DIR | DIR | per-profile | Resolve symbol references (lexical binding, library resolution) |
| Analyze | `A` | DIR | DIR | per-profile | Infer types, resolve overloads, validate semantics, record instances |
| Elaborate | `E` | DIR | canonical DIR | per-profile | Canonicalize DIR: patterns→decision trees, tree literals→calls, etc. |

### Middle-End

The middle-end performs comptime execution, lowers DIR to MIR, and runs verification and optimization.
This region may be skipped for targets that don't require low-level IR (e.g., JS/TS transpilation).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Execute | `X` | DIR | DIR | Execute comptime code via internal MIR and patch DIR |
| Lower | `M` | DIR | MIR | Lower patched DIR to MIR (monomorphization, layouts, RTTI as needed) |
| Verify | `V` | MIR | MIR | Verify and flow-check MIR (safety, borrowing, control flow) |
| Optimize | `O` | MIR | MIR | Optimization passes |

### Back-End

The back-end generates target artifacts from DIR (for JS/TS) or MIR (for native/WASM).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Generate | `G` | DIR/MIR | artifacts | Generate target code (JS/TS from DIR, native from MIR) |
| Link | `K` | artifacts | linked | Link artifacts into final output |
| Emit | `W` | linked | files | Write linked output to disk |

### Lint

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Lint | `L` | DIR | diagnostics | Lint the program (runs alongside the pipeline) |

## Representations

The compiler uses three main intermediate representations.

| Representation | Full Name | Description |
|----------------|-----------|-------------|
| AST | Abstract Syntax Tree | Untyped syntax tree, close to source text (~=CST) |
| DIR | Destack IR | Semantic IR with symbols, scopes, and types (base, canonical, patched) |
| MIR | Machine IR | Monomorphic, target-aware IR for comptime execution, optimization, and native codegen |

## Layout

The compiler is organized into modules corresponding to each phase.

| Path | Description | Source |
|------|-------------|--------|
| `compile/` | Compiler orchestration, task queue, and worker threads | [src/compile/](src/compile/) |
| `import/` | Import and parse source into AST | [src/import/](src/import/) |
| `bind/` | Bind AST to DIR; declare symbols/scopes; syntactic desugaring | [src/bind/](src/bind/) |
| `resolve/` | Resolve symbol references | [src/resolve/](src/resolve/) |
| `analyze/` | Type inference, checking, and overload resolution | [src/analyze/](src/analyze/) |
| `elaborate/` | Post-analysis transforms (patterns, trees, etc.) | [src/elaborate/](src/elaborate/) |
| `execute/` | Execute comptime code and patch DIR | [src/execute/](src/execute/) |
| `lower/` | Lower DIR to MIR | [src/lower/](src/lower/) |
| `verify/` | Verify and flow-check MIR | [src/verify/](src/verify/) |
| `optimize/` | Optimize MIR | [src/optimize/](src/optimize/) |
| `generate/` | Generate artifacts from DIR/MIR | [src/generate/](src/generate/) |
| `link/` | Link artifacts | [src/link/](src/link/) |
| `emit/` | Write output files to disk | [src/emit/](src/emit/) |
| `lint/` | Lint rules and diagnostics | [src/lint/](src/lint/) |
| `unbind/` | DIR → AST utilities | [src/unbind/](src/unbind/) |
| `tests/` | Compiler test scaffolding | [src/tests/](src/tests/) |

## Tasks

The compiler uses a parallel task system for concurrent compilation.
Each phase defines tasks that can yield on dependencies and resume when satisfied.
Tasks are identified by phase letter and sub-code (e.g., `TI001` for Import task 1).
See `compile/task.rs` for task definitions and `compile/queue.rs` for the task queue.
The compiler uses versions (file/module/artifact) to track changes and dependencies between phases.

## Builtins

The compiler loads language builtins from `language/builtin/` as needed based on target configuration.
See [builtin/README.md](../builtin/README.md) for the full structure.

## Profiles and Targets

A **profile** represents a semantic configuration, essentially, a "comptime world" that determines which symbols exist and how types resolve.
A **target** represents a build output, with specific settings for code generation, optimization, and output paths.

**Profile identity** is determined by:
- `output`: OutputFormat (js, ts, wasm, native)
- `runtime`: Runtime (browser, node, deno, bun, wasm-js, wasm-wasi, native-hosted, native-freestanding, native-embedded)
- `platform`: Platform (web, windows, macos, linux, ios, android, wasi, bare-metal, universal)
- `lib`: Normalized library set (e.g., `["esnext", "dom"]`)
- `debug`: Debug flag (affects `import.meta.debug`)
- `env`: Comptime environment snapshot (for `import.meta.env`)
- `flags`: Semantic restriction flags (`no_any`, `no_managed`, `no_exceptions`, etc.)

Generalizing profiles from targets allows sharing work: if two targets use the same profile, they share the canonical DIR and only diverge at code generation (i.e., they have the same canonical profile-dependent DIR but different target-specific MIRs).
The compiler also maintains a shared comptime target used by Execute to evaluate static code; it is not tied to any specific output target.
