# Destack Compiler

The Destack compiler takes JavaScript, TypeScript and Destack sources and translates them into _some_ artifacts via a classic multi-phase compilation pipeline.

## Pipeline

Like most compilers, the Destack compiler has three main regions: 
 1. Front-end (`.(ds|ts|tsx|js|jsx)` → typed DIR)
 2. Middle-end (DIR → optimized MIR)
 3. Back-end (DIR/MIR → artifacts).
(For JS/TS targets, the middle-end may be skipped entirely.)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                FRONT-END                                    │
│                                                                             │
│   source ───► Import ───► Bind ───► Resolve ───► Analyze ───► Elaborate     │
│                 │          │           │            │             │         │
│    Text        AST        DIR       Symbols       Types       Instances     │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                MIDDLE-END                                   │
│                                                                             │
│                   Lower ───────► Verify ───────► Optimize                   │
│                     │              │                │                       │
│                    MIR            CFG           Better MIR                  │
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
Phases are grouped into regions and identified by a single letter for tracing and diagnostics.

### Front-End

The front-end transforms source text into typed, monomorphized DIR.

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Import | `I` | Text | AST | Parse source into abstract syntax tree |
| Bind | `B` | AST | DIR | Create symbols, scopes, initial DIR structure |
| Resolve | `R` | DIR | DIR | Resolve symbol references (lexical binding) |
| Analyze | `A` | DIR | DIR | Type inference, type checking, overload resolution |
| Elaborate | `E` | DIR | DIR | Monomorphize generics, evaluate comptime |

### Middle-End

The middle-end lowers DIR to MIR and performs verification and optimization.
This region may be skipped for targets that don't require low-level IR (e.g., JS/TS transpilation).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Lower | `L` | DIR | MIR | Lower high-level DIR to machine-level IR |
| Verify | `V` | MIR | MIR | Verify and flow-check MIR (safety, borrowing, control flow) |
| Optimize | `O` | MIR | MIR | Optimization passes |

### Back-End

The back-end generates target artifacts from DIR (for JS/TS) or MIR (for native/WASM).

| Phase | Letter | Input | Output | Description |
|-------|--------|-------|--------|-------------|
| Generate | `G` | DIR/MIR | artifacts | Generate target code (JS/TS from DIR, native from MIR) |
| Link | `K` | artifacts | linked | Link artifacts into final output |
| Emit | `M` | linked | files | Write linked output to disk |

## Representations

The compiler uses three main intermediate representations.

| Representation | Full Name | Description |
|----------------|-----------|-------------|
| AST | Abstract Syntax Tree | Untyped syntax tree, close to source text |
| DIR | Data-level IR (Destack IR) | Typed semantic IR with symbols, scopes, and types |
| MIR | Machine-level IR | Low-level IR for optimization and native codegen |

## Layout

The compiler is organized into modules corresponding to each phase.

| Path | Description | Source |
|------|-------------|--------|
| `compile/` | Compiler orchestration, task queue, and worker threads | [src/compile/](src/compile/) |
| `import/` | Import and parse source into AST | [src/import/](src/import/) |
| `bind/` | Bind AST to DIR, declare symbols and scopes | [src/bind/](src/bind/) |
| `resolve/` | Resolve symbol references | [src/resolve/](src/resolve/) |
| `analyze/` | Type inference, checking, and overload resolution | [src/analyze/](src/analyze/) |
| `elaborate/` | Monomorphize and desugar DIR | [src/elaborate/](src/elaborate/) |
| `lower/` | Lower DIR to MIR | [src/lower/](src/lower/) |
| `verify/` | Verify and flow-check MIR | [src/verify/](src/verify/) |
| `optimize/` | Optimize MIR | [src/optimize/](src/optimize/) |
| `generate/` | Generate artifacts from DIR or MIR | [src/generate/](src/generate/) |
| `link/` | Link artifacts | [src/link/](src/link/) |
| `emit/` | Write output files to disk | [src/emit/](src/emit/) |
| `tests/` | Compiler tests | [src/tests/](src/tests/) |

## Tasks

The compiler uses a parallel task system for concurrent compilation.
Each phase defines tasks that can yield on dependencies and resume when satisfied.
Tasks are identified by phase letter and sub-code (e.g., `TI001` for Import task 1).
See `compile/task.rs` for task definitions and `compile/queue.rs` for the task queue.
