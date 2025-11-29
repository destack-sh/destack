# compiler

Destack compiler.
Multi-phase compilation pipeline: binding, resolution, validation, lowering, and optimization.

## Layout

```
src/
├── compile/     Compiler orchestration and task queue
├── import/      Import processing
├── bind/        Name binding (AST → symbols)
├── resolve/     Type resolution
├── validate/    Type checking and validation
├── elaborate/   Type elaboration
├── analyze/     Static analysis
├── lower/       Lowering (AST → DIR)
├── build/       Build artifacts
├── link/        Module linking
├── optimize/    Optimization passes
├── execute/     Execution
└── tests/       Compiler tests
```

