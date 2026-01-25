# Analyze

Analyze transforms resolved DIR into fully typed and semantically validated DIR.
It elaborates type-level declarations, infers value types, resolves overloads, and records the
results for downstream passes.

# Overview

The goal is production-grade TypeScript compatibility with Destack extensions.
Analyze infers types, checks assignability, resolves overloads, and records Instances and Resolutions.
Flow-sensitive typing ensures narrowing matches TypeScript semantics.

Analyze also evaluates **static expressions**—a restricted subset of expressions that can be folded
without executing user code. These are needed for static parameters and other type-driven constructs
that must be known during Analyze.

## Pipeline

Analyze sits between Resolve and Elaborate in the per-profile part of the pipeline.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ANALYZE PIPELINE                               │
│                                                                             │
│  Stages:  Resolve ───► Analyze ───► Elaborate                               │
│  Output:  symbols      types       canonical DIR                            │
│                                                                             │
│                    (all per-profile)                                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Inputs**: Resolved DIR plus explicit type annotations.
**Outputs**: Fully typed DIR plus Instances and Resolutions.

Analyze runs in five internal stages with a clear contract:
1. **Declare**: evaluate `Type::Unevaluated` to a local fixpoint, elaborate type-level declarations, register instance/value shapes plus lineages and extensions, and record signature types for functions and methods
2. **Export**: compute export surface types using local-only surface inference and declared types
3. **Infer**: infer value types and function bodies, resolve overloads, solve constraints, and apply flow narrowing without mutating declared shapes or lineages
4. **Capture**: resolve closure capture sets after inference
5. **Validate**: run semantic validation checks across declarations, parameters, and members

# Outputs

Analyze records results in the **TypeTable** (one per module).
TypeTable is the single source of truth for typing and resolution data.
Elaborate and Lower rely on these results and don't recompute anything.

**What TypeTable stores:**
- **Declared types**: From explicit annotations (`: T`)
- **Inferred types**: Computed for all reachable expressions
- **Signature types**: Declared function or method signatures, refined by inference
- **Instance types**: The shape of declared types
- **Value types**: The type a symbol has at use sites
- **Instances**: Concrete instantiations of generic declarations
- **Resolutions**: Overload selection results for calls and member access
- **Lineages**: Resolved inheritance chains for nominal types
- **Extensions**: Resolved extension declarations

During analysis, we also maintain an **InferTable** with inference variables and constraints.
This gets resolved before Elaborate and isn't consumed by later passes.
Declared types come from explicit annotations and _local_ surface inference for exported values and
type queries. Inferred types are computed for *all* reachable expressions (and some unreachable ones).

Surface inference uses the regular inference engine with caching disabled and no flow narrowing.
It only consults declared types and the local module syntax.

Capture results are recorded in the **CaptureTable** and consumed by Lower.
Validate produces diagnostics only and does not introduce new tables.

# This Binding

Member methods have an implicit `this` binding derived from the receiver type.
Non-member functions and lambdas require an explicit `this` parameter to use `this`.
Lambdas inside methods capture the lexical `this` unless `@capture` overrides it.

# Type Inference

Analyze uses bidirectional typing.
Inferred types flow *out* of expressions; expected types flow *in* from context.
This is what enables contextual typing of lambdas, object literals, and patterns.
(It's also moderately annoying to implement correctly and performantly, but alas, ergonomics.)

**Sources of expected types:**
- Variable and field annotations
- Parameter types for call arguments
- Return types for return expressions
- Contextual types for lambdas and object literals

Inference variables act as placeholders for unknown types.
Constraints get collected during assignments, calls, returns, and pattern bindings.
The solver resolves inference variables to concrete types (or emits errors).

# Constraint Solving

Analyze collects constraints and solves them in a deterministic order.
Constraints are normalized by expanding unions and intersections before solving.

**Constraint kinds:**
1. **Equality**: For explicit annotations and literal inference
2. **Subtype**: For assignability and argument→parameter flows
3. **Join**: For merged control flow environments
4. **Instantiation**: For static parameters and generic inference
5. **Conditional**: For flow-based narrowing
6. **Candidate groups**: For overload selection and ambiguity resolution

**Solver order:**
1. Collect constraints for a full expression tree or function body
2. Normalize types and expand unions/intersections
3. Solve equality constraints (reduces variable count)
4. Solve subtype constraints with variance-aware propagation
5. Solve candidate groups by best signature selection
6. Choose defaults for remaining inference variables
7. Commit final types to TypeTable and emit errors for conflicts


# Flow Typing

Analyze performs flow-sensitive typing using a control flow graph.
Each function has a CFG, and each block has a type environment.
Guard expressions narrow types on outgoing edges.
Merge points combine environments by union or a common supertype.
Unreachable paths are tracked for return, throw, break, and continue.

This matches TypeScript narrowing semantics.
Pattern matching and match guards are part of this flow model.

```
         ┌─────────┐
         │  entry  │
         └────┬────┘
              │
              ▼
         ┌─────────┐
         │ block A │──────┐
         └────┬────┘      │
              │           │
              ▼           ▼
         ┌─────────┐ ┌─────────┐
         │ block B │ │ block C │
         └────┬────┘ └────┬────┘
              │           │
              └─────┬─────┘
                    ▼
              ┌─────────┐
              │  merge  │
              └────┬────┘
                   │
                   ▼
              ┌─────────┐
              │  exit   │
              └─────────┘
```

**Flow environment rules:**
1. A guard creates a refined environment for each outgoing edge.
2. A merge combines environments per symbol.
3. Unreachable blocks carry no environment and don't contribute to merges.

**Narrowing operators:**

| Guard | True branch | False branch |
|-------|-------------|--------------|
| `x == null` | `x: null \| undefined` | `x: Exclude<T, null \| undefined>` |
| `x === null` | `x: null` | `x: Exclude<T, null>` |
| `typeof x == "string"` | `x: string` | `x: Exclude<T, string>` |
| `x instanceof C` | `x: C` | `x: Exclude<T, C>` |
| `"k" in x` | `x: T & { k: unknown }` | `x: Exclude<T, { k: unknown }>` |
| `x is T` | `x: T` | `x: Exclude<U, T>` |

# Resolutions

Analyze resolves member access and overloads at use sites.
Member access selects a member symbol from a type, including extensions.
Call and operator sites choose the best overload by assignability.

**Resolution kinds:**
- **Builtin**: Primitive operations (no function call needed)
- **Static**: Single known target symbol (may still dispatch via vtable)
- **Dynamic**: Union-based dispatch where different variants call different symbols

Resolutions are recorded in the TypeTable and never recomputed later.

**Overload selection order:**
1. Filter candidates by arity and static parameter compatibility
2. Infer static arguments from call site types if needed
3. Check assignability for each candidate in order of specificity
4. Pick the most specific candidate or report ambiguity
5. Record a Static or Dynamic Resolution at the call site node

# Instances ("Generics")

Analyze creates Instances for statically parameterized declarations.
Static arguments can be explicit (`foo<T>()`) or inferred from usage.

If static arguments appear on both a member expression and a call expression, Analyze reports a ConflictingStaticArguments error.
Call static arguments drive inference for that call.
Member static arguments stay attached to the member expression Instance.

The Instance stores a flattened list of static arguments for monomorphization.
Lower needs this to produce specialized MIR.

**Instance creation rules:**
1. Inherited static arguments come first
2. Own static arguments come last
3. The Instance is stored on the node that introduced it

# TypeScript Compatibility

Analyze supports TypeScript-style type operators and literal inference.
Type operators include `keyof`, `typeof`, indexed access, conditional types, and mapped types; all
the usual and fancy TS stuff is supported.

# Builtin Types

Library types ("canonical symbols") like `Promise`, `Iterator`, `Readonly`, `Pick`, and `Record` are resolved from ambient libs and cached per profile.
This keeps TypeScript compatibility while preserving explicit module ownership.

# Module Boundaries

Analyze keeps a TypeTable per module.
Inference is local and never requires whole-program analysis.

Exported bindings publish an **export inference** summary of their declared or locally inferred types.
Other modules import that summary instead of inferring across module boundaries.
Inference cycles across modules are forbidden: if an exported surface cannot be inferred without
depending on another module’s inferred types, it must be explicitly annotated.

When an expression references a symbol from another module, Analyze copies the exported type into
the local TypeTable. Structural types are copied recursively; nominal references retain their
GlobalSymbolId identity. Inference variables from remote modules are replaced with explicit
`unknown` to keep inference local.

# Tests

Primary coverage lives in `language/test/fixtures/specification/`.
These fixtures validate inference and compatibility across language features.
Unit tests in Analyze cover targeted edge cases and regressions.

```bash
cargo test --release -p destack_compiler           # compiler tests
cargo test --release -p destack_test --test specification  # spec suite
```
