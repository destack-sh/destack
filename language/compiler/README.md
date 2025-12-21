# Destack Compiler

The Destack compiler takes JavaScript, TypeScript and Destack sources (`.(ds|ts|tsx|js|jsx)`) and by the power of magic and the art of computer science transforms them into executable artifacts like (`.(js|ts|wasm|o)`) with a modern-ish multi-phase compilation pipeline.

## Pipeline

Like most compilers, the Destack compiler has three main regions:
 1. Front-end (source `.(ds|ts|tsx|js|jsx)` → typed, elaborated "canonical" DIR)
 2. Middle-end (target-independent "canonical" DIR → target-specific "canonical" MIR)
 3. Back-end ("canonical" DIR/MIR → emitted artifacts depending on target).
(For JS/TS targets, the middle-end may be skipped partially or entirely, depending on comptime requirements.)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                FRONT-END                                    │
│                                                                             │
│   source ───► Import ───► Bind ───► Resolve ───► Analyze ───► Elaborate     │
│      │           │         │           │            │             │         │
│    Text        AST        DIR       Symbols       Types     Canonical DIR   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                MIDDLE-END                                   │
│                                                                             │
│         Lower ───────► Verify ───────► Execute ───────► Optimize            │
│           │              │                │                │                │
│          MIR            CFG           Comptime       Canonical MIR          │
│                                                                             │
│              (may be skipped for some targets like JS/TS)                   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                  BACK-END                                   │
│                                                                             │
│              Generate ────────► Link ────────► Emit                         │
│                  │                │               │                         │
│             "Artifacts"       "Linked"        "Files"                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Phases

Each phase transforms or enriches the `Program`.
Phases are identified by a single letter for tracing and diagnostics.
(We also do linting in the compiler, but that's not really a classical "region".)

### Front-End

The front-end transforms source text into typed, elaborated ("canonical") DIR.

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Import | `I` | Text | AST | Parse source into abstract syntax tree |
| Bind | `B` | AST | DIR | Create DIR with symbols and scopes; desugar syntactic forms (`+=`, `++`, etc.) |
| Resolve | `R` | DIR | DIR | Resolve symbol references (lexical binding) |
| Analyze | `A` | DIR | DIR | Infer types, resolve overloads, validate semantics |
| Elaborate | `E` | DIR | DIR | Post-analysis transforms: patterns→decision trees, tree literals→calls, etc. |

### Middle-End

The middle-end lowers DIR to MIR and performs verification, comptime execution, and optimization.
This region may be skipped for targets that don't require low-level IR (e.g., JS/TS transpilation).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Lower | `M` | DIR | MIR | Lower high-level DIR to machine-level IR |
| Verify | `V` | MIR | MIR | Verify and flow-check MIR (safety, borrowing, control flow) |
| Execute | `X` | MIR | MIR | Execute comptime code and substitute results |
| Optimize | `O` | MIR | MIR | Optimization passes |

### Back-End

The back-end generates target artifacts from DIR (for JS/TS) or MIR (for native/WASM).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Generate | `G` | DIR/MIR | artifacts | Generate target code (JS/TS from DIR, native from MIR) |
| Link | `K` | artifacts | linked | Link artifacts into final output |
| Emit | `W` | linked | files | Write linked output to disk |

## Representations

The compiler uses three main intermediate representations.

| Representation | Full Name | Description |
|----------------|-----------|-------------|
| AST | Abstract Syntax Tree | Untyped syntax tree, close to source text (~=CST) |
| DIR | Destack IR | Typed semantic IR with symbols, scopes, and types |
| MIR | Machine IR | Low-level IR for optimization and native codegen |

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
| `lower/` | Lower DIR to MIR | [src/lower/](src/lower/) |
| `verify/` | Verify and flow-check MIR | [src/verify/](src/verify/) |
| `execute/` | Execute comptime code | [src/execute/](src/execute/) |
| `optimize/` | Optimize MIR | [src/optimize/](src/optimize/) |
| `generate/` | Generate artifacts from DIR/MIR | [src/generate/](src/generate/) |
| `link/` | Link artifacts | [src/link/](src/link/) |
| `emit/` | Write output files to disk | [src/emit/](src/emit/) |
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