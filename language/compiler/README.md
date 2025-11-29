# compiler

Destack compiler.
Multi-phase compilation pipeline: binding, resolution, validation, lowering, and optimization.

## Layout

| Path | Purpose | Description |
| --- | --- | --- |
| `compile/` | Orchestration | Compiler orchestration and task queue. |
| `import/` | Parsing | Import and parse source into AST. |
| `bind/` | Binding | Bind, lower and declare AST source into DIR. |
| `resolve/` | Resolution | Resolve symbols, scopes and types in DIR. |
| `validate/` | Validation | Validate and type-check DIR. |
| `elaborate/` | Elaboration | Elaborate, desugar and monomorphize DIR. |
| `lower/` | Lowering | Lower the DIR into MIR. |
| `analyze/` | Analysis | Analyze and flow-check MIR. |
| `optimize/` | Optimization | Optimize the MIR. |
| `execute/` | Execution | Execute MIR statically. |
| `build/` | Building | Build the MIR into artifacts. |
| `link/` | Linking | Link built artifacts into final output. |
| `tests/` | Tests | Compiler tests. |
