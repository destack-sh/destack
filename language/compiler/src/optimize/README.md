# MIR Optimization

Semantic-aware transformations on MIR before code generation.
Runs after Lower, before Generate.

## Overview

```
... → DIR → Lower → MIR → Optimize → Generate → ...
                             │
                             ├─ Verify (safety, borrowing, control flow)
                             ├─ Scalar (constant fold, CSE, copy prop, ...)
                             ├─ Interprocedural (inline, dead function elim, ...)
                             ├─ Memory (escape analysis, stack promote, SROA, ...)
                             ├─ Loop (LICM, unroll, ...)
                             └─ Type-Based (devirtualize, bounds check elim, ...)
```

The Optimize phase runs verification passes early, then executes optimization passes.

Optimize performs transformations taking maximal advantage of high-level semantic information.
The backend handles low-level optimizations such as register allocation, instruction selection, and peephole passes.

## Optimization Levels

| Level | Use Case | What Runs |
|-------|----------|-----------|
| `O0` | Debug | Verification only |
| `O1` | Comptime, dev | Verification + local scalar + SSA/memory canonicalization + type cleanup |
| `O2` | Release | O1 + global scalar, memory, and loop optimizations |
| `O3` | Hot paths | O2 + aggressive loop, vectorization, and interprocedural transforms |

---

## Analyses

Analyses compute properties of the MIR without modifying it.
Passes request analyses from the cache; the cache computes them lazily and invalidates them when passes modify the MIR.
Analysis results are shared across passes until invalidated.

### Dependency Graph

```
           ┌─────────┐
           │   cfg   │
           └────┬────┘
                │
   ┌────────────┼───────────┬───────────┬──────────────┬──────────┐
   │            │           │           │              │          │
   ▼            ▼           ▼           ▼              ▼          ▼
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌──────────┐ ┌──────────┐
│ domtree  │ │ postdom  │ │ liveness │ │ constant-prop│ │ ownership│ │  range   │
└────┬─────┘ └──────────┘ └────┬─────┘ └──────────────┘ └──────────┘ └──────────┘
     │                         │
     ▼                         ▼
  ┌───────┐               ┌────────┐
  │ loops │               │ borrow │
  └───┬───┘               └────────┘
      ▼
┌──────────────────┐
│ scalar-evolution │
└──────────────────┘

┌──────────┐
│  alias   │
└──────────┘

┌──────────┐
│ lifetime │
└──────────┘
```

### Analysis Reference

| ID | Name | Scope | Done | Depends On | Description |
|----|------|-------|------|------------|-------------|
| `cfg` | ControlFlowGraph | function | ✓ | — | Predecessors and successors for each block |
| `domtree` | DominatorTree | function | ✓ | cfg | Dominance relationships, immediate dominators |
| `postdomtree` | PostDominatorTree | function | ✓ | cfg | Post dominance for control dependence analysis |
| `loops` | LoopAnalysis | function | ✓ | domtree | Natural loops, headers, latches, nesting depth |
| `liveness` | LivenessAnalysis | function | ✓ | cfg | Which values are live at each program point |
| `constant-propagation` | ConstantPropagation | function | ✓ | cfg | Constant values per block using SSA and block parameters |
| `reaching-defs` | ReachingDefinitions | function | | cfg | Which local definitions reach each use |
| `available-exprs` | AvailableExpressions | function | | domtree | Which expressions are available at each point |
| `alias` | AliasAnalysis | function | ✓ | — | May-alias and must-alias relationships |
| `memory-ssa` | MemorySSA | function | | domtree, alias | Memory versioning for precise load/store analysis |
| `callgraph` | CallGraph | module | | — | Which functions call which, with call sites |
| `escape` | EscapeAnalysis | module | | callgraph | Which allocations escape their scope |
| `ownership` | OwnershipAnalysis | function | ✓ | cfg | Value ownership and move/copy semantics |
| `borrow` | BorrowAnalysis | function | ✓ | liveness | Active borrows across control flow |
| `lifetime` | LifetimeAnalysis | function | ✓ | — | Lifetime bounds for borrowed returns |
| `type-flow` | TypeFlowAnalysis | function | | domtree | Concrete types at each point (for devirtualization) |
| `scalar-evolution` | ScalarEvolution | function | ✓ | loops | Symbolic expressions for induction variables and trip counts |
| `range` | RangeAnalysis | function | ✓ | cfg | Numeric value ranges (for bounds check elimination) |
| `dependence` | DependenceAnalysis | function | | scalar-evolution, alias | Memory dependence between loop iterations |

---

## Passes

Passes transform the MIR to improve performance or reduce code size.
Each pass declares which analyses it requires and which it invalidates.

Pass categories:
- **Verify**: correctness checks and required transformations (run early in the pipeline)
- **Scalar**: value-level transforms within functions
- **Interprocedural**: cross-function analysis and transforms
- **Memory**: allocation, load/store, aliasing optimizations
- **Loop**: loop-specific transforms
- **Type**: optimizations requiring high-level type information

### Verify (V)

Verification passes ensure semantic correctness and insert required operations.
These run before optimization passes.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `move-check` | MoveCheck | function | V | ✓ | ownership | Verify move semantics: no use-after-move for linear types, copy semantics for trivial types |
| `borrow-check` | BorrowCheck | function | V | ✓ | cfg, liveness, borrow, alias, lifetime | Verify borrow rules, exclusivity, and lifetime annotations |
| `stack-check` | StackCheck | function | V | ✓ | cfg, lifetime | Verify stack safety: no returns of references to locals, no stack pointer escapes |
| `drop-insert` | DropInsert | function | V | ✓ | cfg, liveness, ownership | Insert `raw.drop`/`stack.drop` at last-use points for owned refs |

### Scalar (S)

Local and global optimizations within a single function.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `constant-fold` | ConstantFold | function | O1 | ✓ | constant-propagation | Evaluate operations on constants at compile time |
| `simplify-cfg` | SimplifyCfg | function | O1 | ✓ | — | Branch folding, jump threading, block merging, unreachable elimination |
| `dead-code-eliminate` | DeadCodeEliminate | function | O1 | ✓ | — | Remove dead instructions via backwards liveness (ADCE) |
| `instruction-combine` | InstructionCombine | function | O1 | ✓ | — | Algebraic simplification (x*1=x, x+0=x, x-x=0, x&0=0, etc.) |
| `copy-propagate` | CopyPropagate | function | O1 | ✓ | — | Replace uses of `v1 = v0` with `v0` directly |
| `local-cse` | LocalCse | function | O1 | ✓ | — | Eliminate redundant computations within a basic block |
| `gvn` | GlobalValueNumbering | function | O2 | ✓ | domtree | Eliminate redundant computations across basic blocks |
| `sccp` | SparseConditionalConstantPropagation | function | O2 | ✓ | — | Aggressive constant propagation with unreachable code detection |
| `reassociate` | Reassociate | function | O2 | | — | Reorder associative operations for better constant folding |
| `sink` | CodeSinking | function | O2 | ✓ | cfg, domtree, loops | Move instructions closer to their uses |
| `hoist` | CodeHoisting | function | O2 | | domtree | Move identical instructions to common dominator |
| `pre` | PartialRedundancyElim | function | O3 | | domtree, available-exprs | Insert computations to make partially redundant expressions fully redundant |
| `tail-call-eliminate` | TailCallEliminate | module | O2 | ✓ | — | Convert tail calls to jumps |
| `correlated-value-prop` | CorrelatedValueProp | function | O2 | | domtree | Use dominating conditions to narrow value ranges |
| `if-convert` | IfConvert | function | O2 | | cfg | Convert simple if-then-else diamonds to select/conditional-move |
| `narrow` | Narrow | function | O2 | | — | Use narrower integer types when upper bits are unused |

### Interprocedural (I)

Cross-function optimizations that require module-level analysis.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `inline` | Inline | module | O2 | | callgraph, loops | Inline function calls based on cost/benefit heuristics |
| `dead-function-eliminate` | DeadFunctionEliminate | module | O2 | | callgraph | Remove functions that are never called |
| `dead-arg-eliminate` | DeadArgEliminate | module | O2 | | callgraph | Remove unused function arguments |
| `ip-constant-prop` | InterproceduralConstantProp | module | O2 | | callgraph | Propagate constant arguments across call sites |
| `argument-promote` | ArgumentPromotion | module | O3 | | callgraph, alias | Pass struct fields as separate arguments |
| `global-dce` | GlobalDeadCodeEliminate | module | O2 | | callgraph | Remove unused globals and their initializers |
| `merge-functions` | MergeFunctions | module | O3 | | — | Merge identical function bodies |
| `partial-inline` | PartialInline | module | O3 | | callgraph, loops | Inline only the hot path of a function |
| `constant-merge` | ConstantMerge | module | O2 | | — | Deduplicate identical constants across module |
| `global-opt` | GlobalOpt | module | O2 | | callgraph | Internalize globals, propagate constants, convert never-written to immutable |
| `function-attrs` | FunctionAttrs | module | O2 | | callgraph | Deduce function attributes (nounwind, noreturn, readonly, noescape) |
| `hot-cold-split` | HotColdSplit | module | O3 | | callgraph, loops | Split functions into hot and cold regions for better code layout |

### Memory (M)

Optimizations for memory allocation and access patterns.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `mem2reg` | Mem2Reg | function | O1 | ✓ | domtree | Promote stack allocations to SSA values |
| `sroa` | ScalarReplacementOfAggregates | function | O1 | ✓ | — | Break aggregates into individual scalar values |
| `load-store-forward` | LoadStoreForwarding | function | O2 | ✓ | domtree, alias | Forward stored values to subsequent loads |
| `dse` | DeadStoreEliminate | function | O2 | ✓ | cfg, alias | Remove stores that are overwritten before being read |
| `stack-promote` | StackPromote | function | O2 | | escape | Convert non-escaping heap allocations to stack |

### Loop (L)

Loop-specific transformations.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `loop-simplify` | LoopSimplify | function | O2 | ✓ | loops, cfg | Canonicalize loops (preheader, single latch, dedicated exits) |
| `loop-rotate` | LoopRotate | function | O2 | ✓ | loops, cfg, domtree | Rotate simple loops (header with no instructions) |
| `licm` | LoopInvariantCodeMotion | function | O2 | ✓ | loops, domtree | Move pure loop-invariant computations to preheader |
| `induction-simplify` | InductionVariableSimplify | function | O2 | | scalar-evolution | Simplify or eliminate derived induction variables |
| `loop-strength-reduce` | LoopStrengthReduce | function | O2 | | scalar-evolution | Replace expensive ops (mul) with cheaper ones (add) |
| `loop-delete` | LoopDelete | function | O2 | ✓ | loops, domtree, constant-propagation | Delete loops that compute nothing useful |
| `loop-unroll` | LoopUnroll | function | O3 | | scalar-evolution | Unroll loops with known or small trip counts |
| `loop-unswitch` | LoopUnswitch | function | O3 | ✓ | loops, domtree | Move loop-invariant conditionals outside the loop |
| `loop-fusion` | LoopFusion | function | O3 | | dependence | Merge adjacent loops with same bounds |
| `loop-interchange` | LoopInterchange | function | O3 | | dependence | Swap loop nesting order for cache locality |
| `loop-distribute` | LoopDistribute | function | O3 | | dependence | Split loops to enable partial vectorization |
| `loop-vectorize` | LoopVectorize | function | O3 | | dependence | Vectorize loop iterations (SIMD) |
| `slp-vectorize` | SlpVectorize | function | O3 | | alias | Vectorize straight-line code (superword parallelism) |

### Type (T)

Optimizations that require high-level type information.
These are MIR-only optimizations that justify having an optimizer above the backend.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `devirtualize` | Devirtualize | function | O2 | | type-flow | Convert virtual calls to direct when concrete type is known |
| `bounds-check-eliminate` | BoundsCheckEliminate | function | O1 | ✓ | range | Remove array bounds checks when provably safe |
| `null-check-eliminate` | NullCheckEliminate | function | O2 | | type-flow | Remove null checks when provably non-null |
| `specialize` | FunctionSpecialize | module | O3 | | callgraph | Create specialized versions for constant arguments |

## Pass Structure

Passes implement the `FunctionPass` or `ModulePass` interface:

```ds
interface FunctionPass extends Pass {
    runOnFunction(
        function: mut mir.Function,
        tree: mut mir.NodeTree,
        context: OptimizationContext,
    ): AnalysisPreservation
}

interface ModulePass extends Pass {
    runOnModule(
        tree: mut mir.NodeTree,
        context: OptimizationContext,
    ): AnalysisPreservation
}
```

The `AnalysisPreservation` return value tells the pass manager which cached analyses are still valid.
Return `AnalysisPreservation.all()` if no changes were made.
Return `AnalysisPreservation.none()` if any cached analysis may be invalidated.

## Pipeline

Target pipelines evolve with new analyses, but the expected layering is:
- `O1`: verify, local scalar cleanup, SSA/memory canonicalization, final cleanup
- `O2`: `O1` + global scalar, memory optimizations, core loop transforms
- `O3`: `O2` + aggressive loop transforms, vectorization, interprocedural passes

Module passes run first, then function passes run on each function.
Pipelines may iterate passes until a fixed point when profitable.
