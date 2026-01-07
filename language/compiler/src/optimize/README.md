# MIR Optimization

Semantic-aware transformations on MIR before code generation.
Runs after Verify, before Generate.

## Overview

```
... → MIR → Verify → Optimize → Generate → ...
                        │
                        ├─ ConstantFold
                        ├─ DeadCodeEliminate
                        ├─ Inline
                        ├─ EscapeAnalyze
                        ├─ StackPromote
                        ├─ Devirtualize
                        ├─ BoundsCheckEliminate
                        └─ ...
```

Optimize performs transformations that need type info or high-level structure:
- Devirtualization (concrete type known from flow analysis)
- Escape analysis (knows allocation semantics)
- Bounds check elimination (knows array lengths from types)
- Inlining decisions (cross-function, profile-aware)

## Optimization Levels

| Level | Use Case | What Runs |
|-------|----------|-----------|
| `O0` | Debug | Nothing |
| `O1` | Comptime, dev | Fast local passes (constant fold, DCE, simplify CFG) |
| `O2` | Release | Full suite (inlining, escape analysis, devirt) |
| `O3` | Hot paths | Aggressive thresholds, loop unrolling |

## Pass Structure

Passes implement the `Pass` interface and declare what analyses they need:

```ts
interface Pass {
    meta(): PassMeta
    runFunction(ctx: OptContext, func: mir.Function): boolean
    runModule(ctx: OptContext, functions: mir.Function[]): boolean
}

interface PassMeta {
    id: string              // e.g., "constant-fold"
    name: string            // e.g., "ConstantFold"
    description: string
    requires: Analysis[]    // analyses this pass needs
    invalidates: Analysis[] // analyses this pass invalidates
}

enum Analysis {
    Dominators,
    Liveness,
    EscapeInfo,
    AliasInfo,
    CallGraph,
}
```

The pass manager caches analyses and invalidates them when passes modify the MIR.

## Profile-Guided Optimization

Profile data is a first-class input to optimization.
Annotations like `@hot`, `@cold`, `@inline`, `@noinline` provide compile-time hints.
Real PGO data from instrumented runs can guide inlining and layout decisions.

---

*Target-specific optimizations (register allocation, instruction selection, peephole) are handled by Cranelift after MIR codegen. Long-term, we may bring these in-house.*