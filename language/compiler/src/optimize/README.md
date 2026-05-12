# MIR Optimization

Make MIR more optimal for execution.

## Optimization Levels

| Level | Use Case | What Runs |
|-------|----------|-----------|
| `O0` | Debug | Canonicalization only |
| `O1` | Dev, fast builds | Local scalar + SSA/memory canonicalization + type cleanup and bounds checks |
| `O2` | Release | O1 + global scalar, memory, and loop optimizations |
| `O3` | Hot paths | O2 + aggressive loop, vectorization, and interprocedural transforms |
| `O4` | Max | O3 + extra fixed point rounds and optional LTO |

## Analyses

```text
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

| ID | Name | Scope | Done | Depends On | Description |
|----|------|-------|------|------------|-------------|
| `cfg` | ControlFlowGraph | function | ✓ | — | Predecessors and successors for each block |
| `domtree` | DominatorTree | function | ✓ | cfg | Dominance relationships, immediate dominators |
| `postdomtree` | PostDominatorTree | function | ✓ | cfg | Post dominance for control dependence analysis |
| `loops` | LoopAnalysis | function | ✓ | domtree | Natural loops, headers, latches, nesting depth |
| `liveness` | LivenessAnalysis | function | ✓ | cfg | Which values are live at each program point |
| `constant-propagation` | ConstantPropagation | function | ✓ | cfg | Constant values per block using SSA and block parameters |
| `reaching-defs` | ReachingDefinitions | function | ✓ | cfg | Which local definitions reach each use |
| `available-exprs` | AvailableExpressions | function | ✓ | cfg | Which expressions are available at each point |
| `alias` | AliasAnalysis | function | ✓ | — | May-alias and must-alias relationships |
| `memory-ssa` | MemorySSA | function | ✓ | domtree, ownership | Memory versioning for precise load/store analysis |
| `callgraph` | CallGraph | module | ✓ | — | Which functions call which, with call sites |
| `callgraph-scc` | CallGraphScc | module | ✓ | callgraph | Strongly connected components for recursion detection |
| `package-callgraph` | PackageCallGraph | package | ✓ | — | Cross-module call edges for a package workset |
| `program-callgraph` | ProgramCallGraph | program | ✓ | — | Cross-package call edges for a program workset |
| `profile` | ProfileSummary | module | | — | Profile counters, hotness, and value profiles |
| `branch-prob` | BranchProbability | function | | cfg, profile | Branch probabilities derived from profiles |
| `block-freq` | BlockFrequency | function | | cfg, branch-prob | Estimated block execution frequencies |
| `effect` | EffectAnalysis | function | | callgraph | Summary of side effects, allocation, and throws |
| `exception-flow` | ExceptionFlowAnalysis | function | | cfg, effect | Which edges can throw or unwind |
| `tbaa` | TypeBasedAliasAnalysis | function | ✓ | type-flow | Alias refinement from static type information |
| `typed-array` | TypedArrayAnalysis | function | | type-flow | Typed array element kinds and bounds facts |
| `string-literal` | StringLiteralAnalysis | function | | constant-propagation | Known string constants and length data |
| `escape` | EscapeAnalysis | module | | callgraph | Which allocations escape their scope |
| `ownership` | OwnershipAnalysis | function | ✓ | cfg | Value ownership and move/copy semantics |
| `borrow` | BorrowAnalysis | function | ✓ | liveness | Active borrows across control flow |
| `lifetime` | LifetimeAnalysis | function | ✓ | — | Lifetime bounds for borrowed returns |
| `type-flow` | TypeFlowAnalysis | function | | domtree | Concrete types at each point (for devirtualization) |
| `scalar-evolution` | ScalarEvolution | function | ✓ | loops | Symbolic expressions for induction variables and trip counts |
| `range` | RangeAnalysis | function | ✓ | cfg | Numeric value ranges (for bounds check elimination) |
| `dependence` | DependenceAnalysis | function | | scalar-evolution, alias | Memory dependence between loop iterations |
| `loop-access` | LoopAccessAnalysis | function | | loops, scalar-evolution, alias, range | Loop memory access patterns and strides |

---

## Passes

Passes transform the MIR to improve performance or reduce code size.
Each pass declares which analyses it requires and which it invalidates.

Pass categories:
- **Scalar**: value-level transforms within functions
- **Interprocedural**: cross-function analysis and transforms
- **Memory**: allocation, load/store, aliasing optimizations
- **Loop**: loop-specific transforms
- **Type**: optimizations requiring high-level type information

### Scalar (S)

Local and global optimizations within a single function.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `constant-fold` | ConstantFold | function | O1 | ✓ | constant-propagation | Evaluate operations on constants at compile time |
| `simplify-cfg` | SimplifyCfg | function | O1 | ✓ | constant-propagation, range | Branch/check/switch folding, edge threading, tail duplication, block merging, unreachable elimination, critical edge splitting |
| `dead-code-eliminate` | DeadCodeEliminate | function | O1 | ✓ | — | Remove dead instructions via backwards liveness (ADCE) |
| `instruction-combine` | InstructionCombine | function | O1 | ✓ | — | Algebraic simplification (x*1=x, x+0=x, x-x=0, x&0=0, etc.) |
| `copy-propagate` | CopyPropagate | function | O1 | ✓ | — | Replace uses of `v1 = v0` with `v0` directly |
| `local-cse` | LocalCse | function | O1 | ✓ | — | Eliminate redundant computations within a basic block |
| `gvn` | GlobalValueNumbering | function | O2 | ✓ | domtree | Eliminate redundant computations across basic blocks |
| `sccp` | SparseConditionalConstantPropagation | function | O2 | ✓ | — | Aggressive constant propagation with unreachable code detection |
| `reassociate` | Reassociate | function | O2 | ✓ | constant-propagation | Reorder associative operations for better constant folding |
| `sink` | CodeSinking | function | O2 | ✓ | cfg, domtree, loops | Move instructions closer to their uses |
| `hoist` | CodeHoisting | function | O2 | ✓ | cfg, domtree | Move identical instructions to common dominator |
| `pre` | PartialRedundancyElim | function | O3 | ✓ | cfg, domtree, available-exprs | Insert computations to make partially redundant expressions fully redundant |
| `tail-call-eliminate` | TailCallEliminate | module | O2 | ✓ | — | Convert tail calls to jumps |
| `correlated-value-prop` | CorrelatedValueProp | function | O2 | ✓ | domtree | Use dominating conditions to narrow value ranges |
| `if-convert` | IfConvert | function | O2 | ✓ | cfg | Convert simple if-then-else diamonds to select/conditional-move |
| `narrow` | Narrow | function | O2 | ✓ | range, ownership | Use narrower integer types when upper bits are unused |
| `guard-eliminate` | GuardEliminate | function | O2 | ✓ | cfg, range | Remove redundant guard and check terminators |
| `value-range-prop` | ValueRangePropagation | function | O2 | ✓ | range | Fold values proven constant by range analysis |
| `path-clone` | PathClone | function | O3 | | cfg, domtree, profile | Clone hot paths to expose constants and simplify control flow |
| `cfg-layout` | CfgLayout | function | O2 | ✓ | cfg, profile | Layout blocks for fallthrough and cache locality |

### Interprocedural (I)

Cross-function optimizations that require module-level analysis.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `inline` | Inline | module | O2 | ✓ | callgraph, loops | Inline function calls based on cost/benefit heuristics |
| `dead-function-eliminate` | DeadFunctionEliminate | module | O2 | ✓ | callgraph | Remove functions that are never called |
| `dead-arg-eliminate` | DeadArgEliminate | module | O2 | ✓ | — | Remove unused function arguments |
| `ip-constant-prop` | InterproceduralConstantPropagation | module | O2 | ✓ | — | Propagate constant arguments across call sites |
| `ip-sccp` | InterproceduralSccp | module | O2 | ✓ | constant-prop | Propagate constants across call edges and prune dead paths |
| `argument-specialize` | ArgumentSpecialize | module | O3 | ✓ | callgraph, constant-prop, profile | Clone callees for constant argument call sites |
| `argument-promote` | ArgumentPromotion | module | O3 | | callgraph, alias | Pass struct fields as separate arguments |
| `global-dead-code-eliminate` | GlobalDeadCodeEliminate | module | O2 | ✓ | — | Remove unused globals and their initializers |
| `merge-functions` | MergeFunctions | module | O3 | | — | Merge identical function bodies |
| `partial-inline` | PartialInline | module | O3 | | callgraph, loops | Inline only the hot path of a function |
| `constant-merge` | ConstantMerge | module | O2 | | — | Deduplicate identical constants across module |
| `global-opt` | GlobalOpt | module | O2 | ✓ | — | Convert never-written globals to immutable and fold constant loads |
| `ip-dce-cleanup` | InterproceduralDceCleanup | module | O2 | ✓ | callgraph, cfg | Prune dead globals and functions after IPO |
| `function-attrs` | FunctionAttrs | module | O2 | ✓ | callgraph | Infer memory effects and call behavior for functions and callsites |
| `hot-cold-split` | HotColdSplit | module | O3 | | callgraph, loops | Split functions into hot and cold regions for better code layout |
| `pgo-inline` | ProfileGuidedInline | module | O2 | | callgraph, profile | Inline based on callsite hotness and value profiles |
| `indirect-call-promotion` | IndirectCallPromotion | module | O3 | | callgraph, profile | Promote hot indirect calls to direct with fallback |
| `pgo-devirtualize` | ProfileGuidedDevirtualize | function | O3 | | type-flow, profile | Speculative devirtualization guarded by profiles |
| `function-specialize-pgo` | FunctionSpecializePGO | module | O3 | | callgraph, profile | Specialize hot callsites with constant arguments |

### Memory (M)

Optimizations for memory allocation and access patterns.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `mem2reg` | Mem2Reg | function | O1 | ✓ | cfg, domtree | Promote stack allocations to SSA values |
| `sroa` | ScalarReplacementOfAggregates | function | O1 | ✓ | constant-propagation | Break aggregates into individual scalar values |
| `load-pre` | LoadPre | function | O2 | ✓ | cfg, domtree, memory-ssa | Insert edge loads to eliminate redundant join loads |
| `store-pre` | StorePre | function | O2 | ✓ | cfg, domtree, memory-ssa | Insert edge stores to remove redundant join stores |
| `load-store-forward` | LoadStoreForwarding | function | O2 | ✓ | domtree, alias, memory-ssa | Forward stored values to subsequent loads |
| `mem-cse` | MemCse | function | O2 | ✓ | alias, memory-ssa | Remove redundant stores that write identical values |
| `store-sink` | StoreSink | function | O2 | ✓ | cfg, memory-ssa | Sink stores to successor edges that use them |
| `dse` | DeadStoreEliminate | function | O2 | ✓ | cfg, alias, memory-ssa | Remove stores that are overwritten before being read |
| `memcpy-opt` | MemcpyOpt | function | O2 | | alias, memory-ssa, constant-propagation | Simplify and merge memcpy, memmove, and memset operations |
| `stack-promote` | StackPromote | function | O2 | | escape | Convert non-escaping heap allocations to stack |
| `gc-barrier-write-elide` | BarrierWriteElide | function | O2 | | alias, effect, escape | Remove redundant GC barrier writes |
| `speculative-load-hoist` | SpeculativeLoadHoist | function | O3 | | domtree, alias, exception-flow, block-freq | Hoist loads speculatively when safe |

### Loop (L)

Loop-specific transformations.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `loop-simplify` | LoopSimplify | function | O2 | ✓ | loops, cfg | Canonicalize loops (preheader, single latch, dedicated exits) |
| `loop-rotate` | LoopRotate | function | O2 | ✓ | loops, cfg, domtree | Rotate simple loops (header with no instructions) |
| `licm` | LoopInvariantCodeMotion | function | O2 | ✓ | loops, domtree, range, alias, memory-ssa | Move loop invariant computations and safe loads to preheader |
| `induction-simplify` | InductionVariableSimplify | function | O2 | ✓ | loops, cfg, scalar-evolution | Simplify or eliminate derived induction variables |
| `loop-strength-reduce` | LoopStrengthReduce | function | O2 | ✓ | loops, cfg, domtree, scalar-evolution, ownership, range | Replace expensive ops (mul) with cheaper ones (add) |
| `loop-delete` | LoopDelete | function | O2 | ✓ | loops, domtree, constant-propagation | Delete loops that compute nothing useful |
| `loop-unroll` | LoopUnroll | function | O3 | ✓ | scalar-evolution | Unroll loops with known or small trip counts |
| `loop-unswitch` | LoopUnswitch | function | O3 | ✓ | loops, cfg, domtree | Move loop-invariant conditionals outside the loop |
| `loop-peel` | LoopPeel | function | O3 | ✓ | loops, cfg, domtree | Peel iterations to simplify guards and expose invariants |
| `loop-versioning` | LoopVersioning | function | O3 | ✓ | loops, cfg, scalar-evolution | Create fast path loops guarded by assumptions |
| `loop-idiom` | LoopIdiomRecognize | function | O2 | ✓ | loops, cfg, scalar-evolution, ownership | Recognize memset style loops |
| `unroll-and-jam` | LoopUnrollAndJam | function | O3 | ✓ | loops, cfg, domtree, scalar-evolution, ownership | Unroll outer loops and jam inner loops |
| `loop-fusion` | LoopFusion | function | O3 | ✓ | loops, cfg, domtree, memory-ssa, alias, constant-propagation | Merge adjacent loops with the same bounds |
| `loop-interchange` | LoopInterchange | function | O3 | ✓ | loops, cfg, domtree, memory-ssa | Swap perfectly nested read only loops |
| `loop-distribute` | LoopDistribute | function | O3 | ✓ | loops, cfg, domtree, memory-ssa, alias | Split loops into disjoint store groups |
| `loop-tiling` | LoopTiling | function | O3 | | loops, scalar-evolution, loop-access, dependence | Tile loops (strip mine) for cache and vectorization |
| `loop-collapse` | LoopCollapse | function | O3 | | loops, scalar-evolution, loop-access, dependence | Collapse perfectly nested loops into a single iteration space |
| `loop-skew` | LoopSkew | function | O3 | | loops, scalar-evolution, dependence | Skew nested loops to satisfy dependence constraints |
| `loop-prefetch` | LoopPrefetch | function | O3 | | loops, loop-access, alias, profile | Insert software prefetches for predictable strides |
| `software-pipeline` | SoftwarePipelining | function | O3 | | loops, dependence, block-freq, profile | Schedule loop operations to overlap iterations |
| `loop-vectorize` | LoopVectorize | function | O3 | | dependence | Vectorize loop iterations (SIMD) |
| `slp-vectorize` | SlpVectorize | function | O3 | | alias | Vectorize straight-line code (superword parallelism) |

### Type (T)

Optimizations that require high-level type information.
These are MIR-only optimizations that justify having an optimizer above the backend.

| ID | Name | Scope | Level | Done | Requires | Description |
|----|------|-------|-------|------|----------|-------------|
| `devirtualize` | Devirtualize | function | O2 | | type-flow | Convert class calls to direct when concrete type is known |
| `bounds-check-eliminate` | BoundsCheckEliminate | function | O1 | ✓ | constant-propagation, cfg, domtree, range | Remove array bounds checks when provably safe |
| `loop-bounds-check-eliminate` | LoopBoundsCheckEliminate | function | O1 | ✓ | loops, cfg, domtree, scalar-evolution, range | Remove loop bounds checks dominated by loop guards |
| `null-check-eliminate` | NullCheckEliminate | function | O2 | | type-flow | Remove null checks when provably non-null |
| `nullability-prop` | NullabilityPropagation | function | O2 | | type-flow, domtree | Propagate non null facts through control flow |
| `type-test-eliminate` | TypeTestEliminate | function | O2 | | type-flow, domtree | Remove redundant type tests and instanceof checks |
| `union-split` | UnionSplit | function | O2 | | type-flow, domtree | Split control flow based on union tags |
| `specialize` | FunctionSpecialize | module | O3 | | callgraph | Create specialized versions for constant arguments |

## Pipeline

Target pipelines evolve with new analyses, but the expected layering is:
- `O1`: verify, SSA/memory canonicalization, light scalar fixed point, type cleanup
- `O2`: `O1` + global scalar fixed point islands around memory, loop, and type transforms
- `O3`: `O2` + more aggressive fixed point islands and loop transforms
- `O4`: `O3` + extra fixed point rounds and LTO based on ltoMode
