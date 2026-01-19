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

## Phases

Each phase transforms or enriches the `Program`.
Phases are identified by a single letter for tracing and diagnostics, and just because that seems kind of nice.
Linting is a separate "phase" that conceptually runs alongside the pipeline.

### Front-End

The "front-end" transforms source text into typed, elaborated ("canonical") DIR.
Import is **profile-independent**, producing shared base DIR (parse, bind symbols/scopes, desugar).
Resolve, Analyze, and Elaborate are **per-profile**, producing canonical DIR for each profile.

| Phase | Letter | Input | Output | Profile | Description |
|-------|--------|-------|--------|---------|-------------|
| Import | `I` | Text | base DIR | — | Parse source into AST; create DIR with symbols and scopes; desugar syntactic forms (`+=`, `++`, etc.) |
| Resolve | `R` | base DIR | DIR | per-profile | Resolve symbol references (lexical binding, library resolution) |
| Analyze | `A` | DIR | DIR | per-profile | Elaborate type declarations, infer value types, resolve overloads, validate semantics, record instances |
| Elaborate | `E` | DIR | canonical DIR | per-profile | Canonicalize DIR: patterns→decision trees, tree literals→calls, etc. |

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

## Structure

The compiler is organized into modules (roughly) corresponding to each phase.

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

The compiler uses a parallel task system for concurrent compilation.
Each phase defines tasks that can yield on dependencies and resume when satisfied.
Tasks are identified by phase letter and sub-code (e.g., `TI001` for Import task 1).
Tasks track file, module, and artifact versions for change detection and dependency tracking.
See `compile/task.rs` for task definitions and `compile/queue.rs` for the task queue.

## Incremental Compilation

Incremental compilation reuses phase outputs keyed by file, module, profile, and target versions.
Edits bump `FileVersion` and propagate to `ModuleVersion`.
Per-profile module signatures are computed after `Analyze` and gate downstream invalidation.
Downstream modules re-`Analyze` only when the signatures they import change.
Task dependencies are recorded through `require_*` calls and resolved by the task queue.
Comptime results are invalidated when either the module version or profile version changes.

### Version Axes

The compiler uses explicit version axes to make incremental invalidation sound.
The version axes are:
- `FileVersion`: changes on any file content update, including virtual edits.
- `ModuleVersion`: changes when a module source changes and invalidates module-scoped caches.
- `ProfileVersion`: changes when dsconfig or tsconfig changes and invalidates profile-scoped caches.
- `PackageVersion`: changes when any module or config in the package changes.
- `TargetId`: identifies target-specific work and is included in target-scoped tasks.

### Task Keys

Each task key includes the versions required to make it safe to reuse.
The task key rules are:
- Import tasks use the `module` stamp (id plus version).
- Resolve builtins and lib tasks use the `profile` stamp (id plus version).
- Resolve module tasks use the `module` and `profile` stamps.
- Analyze tasks use the `module` and `profile` stamps.
- Elaborate and Execute tasks use the `module` and `profile` stamps.
- Lower, Optimize, and Generate tasks use the `module` and `profile` stamps plus `target_id`.
- Lint module tasks use the `module` and `profile` stamps.
- Lint package tasks use the `package` stamp (id plus version).
- Link and Emit tasks use the `package` stamp plus `target_id`.

### Stale Task Guards

Tasks re-check versions at the start of processing and before committing writes.
If versions mismatch, tasks return early without mutating program state or diagnostics.
This prevents in-flight stale tasks from clobbering newer incremental state.

### LSP Update Semantics

LSP text updates are applied only if the document version is monotonic.
Stale updates are ignored and logged at debug level.

### Delete and Rename Semantics

Delete and rename events are treated as remove plus add, even across packages.
Module graphs and signatures are updated to reflect removed modules and new module ids.

### Task Pruning

Long-lived daemons prune completed tasks to bound memory growth.
Pruning is driven by version epochs so that stale tasks can be safely discarded.

### Module Signatures

Module signatures summarize the externally visible surface of a module for a specific profile.
Signatures include exported names, exported type shapes, module bindings, and global augmentations.
Comptime outputs contribute to signatures when they affect exported values or types.
Signatures are stored as stable hashes and compared to decide downstream work.

### Dynamic Evaluation

Dynamic evaluation compiles input into a synthetic module under a dynamic execution policy.
The compiler treats dynamic modules like REPL cells for dependency tracking and caching.
Dynamic evaluation is never available during comptime execution.

## Caching

Caching is layered to keep the straight line pipeline fast while avoiding recomputation.
In-memory caches live on the `Program` and are keyed by module, profile, and target versions.
On-disk caches are keyed by compiler version, file version, profile version, config hash, and target hash.
Config hashes are derived from canonical dsconfig JSON with tooling-only sections excluded.
Target hashes include the resolved target config and target id.
Cache format mismatches are treated as cache misses and must not fail compilation.
Cache eviction is policy driven and should not silently mask version mismatches.
The compiler, daemon, and LSP share a single canonical cache format for all reusable artifacts.
Consumer-specific metadata lives in sidecar files keyed by the same cache key.

## Builtins

The compiler loads language builtins from `language/builtin/` as needed based on target configuration.
See [builtin/README.md](../builtin/README.md) for the full structure, basically, it's a bunch of .d.ts libs plus our own custom core / std stuff, and the main native lib.

## Profiles and Targets

A **profile** represents a semantic configuration, essentially, a "comptime world" that determines which symbols exist and how types resolve.
A **target** represents a build output, with specific settings for code generation, optimization, and output paths.

**Profile identity** is ~everything that might affect the middle-end by:
- `output`: OutputFormat (js, ts, wasm, native)
- `runtime`: Runtime (browser, node, deno, bun, wasm-js, wasm-wasi, native-hosted, native-freestanding, native-embedded)
- `platform`: Platform (web, windows, macos, linux, ios, android, wasi, bare-metal, universal)
- `lib`: Normalized library set (e.g., `["esnext", "dom"]`)
- `debug`: Debug flag (affects `import.meta.debug`)
- `env`: Comptime environment snapshot (for `import.meta.env`)
- `flags`: Semantic restriction flags (`no_any`, `no_managed`, `no_exceptions`, etc.)

(The compiler also maintains a shared comptime target used by Execute to evaluate static code; it is not tied to any specific output target. We need to run comptime on *something*.)
