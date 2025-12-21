# Analyze: Type Inference and Resolution

Analyze transforms resolved DIR into fully typed and semantically validated DIR.
Analyze computes Types, Instances, Resolutions, and flow aware narrowing.
Elaborate consumes Analyze output to produce canonical DIR for Generate and Lower.
See [compiler/README.md](../README.md) for the full pipeline.
See [lower/README.md](../lower/README.md) for how canonical DIR is consumed.

---

# Overview

## Objectives

The goal is production grade TypeScript compatibility with Destack extensions.
Analyze infers types, checks assignability, resolves overloads, and records Instances and Resolutions.
Flow sensitive typing ensures narrowing matches TypeScript semantics.
The system is deterministic and stable under refactors and large code bases.

## Pipeline Context

Analyze sits between Resolve and Elaborate.
Resolve binds symbol references in DIR.
Analyze computes types and resolutions based on those symbols.
Elaborate canonicalizes DIR using Analyze results.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   Bind ───► Resolve ───► Analyze ───► Elaborate ───► Generate or Lower  │
│               │            │              │                             │
│            symbols       types        canonical                         │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**Inputs** are resolved DIR plus explicit type annotations.
**Outputs** are fully typed DIR plus Instances and Resolutions.

Detailed contract:

```
Resolved DIR
    │
    ├─► Infer Types and Constraints
    │
    ├─► Solve and Commit Types
    │
    ├─► Resolve Members and Overloads
    │
    ├─► Create Instances
    │
    └─► Record Flow Narrowing
```

---

# Outputs and State

Analyze records results in the module TypeTable.
TypeTable is the single source of truth for typing and resolution data in a module.
Elaborate and Lower rely on these results and do not recompute types or resolve overloads again.

High level TypeTable shape:

```ds
struct TypeTable {
    module_id: ModuleId

    types: Arena<Type>
    declared_type_by_node_id: Map<GlobalNodeId, LocalTypeId>
    inferred_type_by_node_id: Map<GlobalNodeId, LocalTypeId>

    instance_type_by_symbol_id: Map<GlobalSymbolId, LocalTypeId>
    value_type_by_symbol_id: Map<GlobalSymbolId, LocalTypeId>

    instances: Arena<Instance>
    instance_by_node_id: Map<GlobalNodeId, LocalInstanceId>

    resolutions: Arena<Resolution>
    resolution_by_node_id: Map<GlobalNodeId, LocalResolutionId>

    lineages: Arena<Lineage>
    lineage_by_symbol_id: Map<GlobalSymbolId, LocalLineageId>

    extensions: Arena<Extension>
    extensions_by_target: Map<GlobalSymbolId, Vec<LocalExtensionId>>

    diagnostics: Vec<AnalyzeDiagnostic>
}
```

Each node can have a **declared Type**, an **inferred Type**, or both.
Each symbol can have an **instance Type** and a **value Type**.
Instances and Resolutions are recorded at the node that triggered them.

**Key invariants**
* Declared Types come only from explicit annotations.
* Inferred Types are computed for all reachable expressions.
* Instance Types describe the shape of a declared type.
* Value Types describe the type a symbol has at use sites.
* Resolutions capture overload selection and are stable across phases.

Analyze also maintains an InferTable during analysis.
InferTable stores inference variables, constraints, and solver state for a module.
InferTable is resolved before Elaborate and is not consumed by later passes.

High level InferTable shape:

```ds
struct InferTable {
    vars: Arena<InferVar>
    constraints: Vec<Constraint>
    varByNodeId: Map<GlobalNodeId, InferVarId>
    varBySymbolId: Map<GlobalSymbolId, InferVarId>
    varByTypeParameter: Map<GlobalSymbolId, InferVarId>
}

struct InferVar {
    lowerBounds: Vec<LocalTypeId>
    upperBounds: Vec<LocalTypeId>
    default: Option<LocalTypeId>
    origin: InferOriginId
    scope: InferScope
}

enum InferOrigin {
    Expression(GlobalNodeId)
    Parameter(GlobalNodeId)
    Return(GlobalNodeId)
    TypeParameter(GlobalSymbolId)
    ConstraintGroup(ConstraintGroupId)
}

struct InferScope {
    owner: GlobalSymbolId
    function_id: Option<GlobalNodeId>
}

enum Constraint {
    Equal { left: LocalTypeId, right: LocalTypeId }
    Subtype { sub: LocalTypeId, sup: LocalTypeId, variance: Variance }
    Join { target: InferVarId, sources: Vec<LocalTypeId> }
    Instantiate { target: InferVarId, generic: LocalTypeId, args: Vec<LocalTypeId> }
    Conditional { guard: LocalTypeId, when_true: LocalTypeId, when_false: LocalTypeId }
    CandidateGroup { id: ConstraintGroupId, options: Vec<Vec<Constraint>> }
}
```

---

# Type Representation

Analyze operates on Type, Instance, and Resolution structures that are stable across passes.
Type records the semantic shape of values and types in DIR.
Instance records a concrete instantiation of a statically parameterized declaration.
Resolution records member selection and overload dispatch results.

High level Type shape:

```ds
enum Type {
    TypeLiteral(TypeLiteral)
    Value(value: LocalTypeId)
    Reference { symbol: GlobalSymbolId, static_arguments: Option<Vec<StaticArgument>> }
    Unevaluated(expression: LocalNodeId)

    InferVar(InferVarId)

    Unary { operator: TypeUnaryOperator, right: LocalTypeId }
    Binary { left: LocalTypeId, operator: TypeBinaryOperator, right: LocalTypeId }

    Mutable { mutability: Mutability, right: LocalTypeId }
    ValueOf { mutability: Option<Mutability>, variance: Option<VarianceBound>, right: LocalTypeId }
    ReferenceOf { mutability: Option<Mutability>, variance: Option<VarianceBound>, right: LocalTypeId }

    ArraySized { element: LocalTypeId, count: LocalNodeId }
    Array { element: Option<LocalTypeId> }
    Tuple { elements: Vec<LocalTypeId> }
    Object { fields: Vec<TypeField> }
    Function {
        asynchrony: Asynchrony
        cardinality: FunctionCardinality
        static_parameters: Vec<LocalTypeId>
        dynamic_parameters: Vec<LocalTypeId>
        return_type: Option<LocalTypeId>
    }

    Union { elements: Vec<LocalTypeId> }
    Intersection { elements: Vec<LocalTypeId> }

    Error
}
```

High level TypeLiteral shape:

```ds
enum TypeLiteral {
    Never
    Any
    Infer
    Undefined
    Unknown
    Void
    Null
    Primitive(PrimitiveType)
    Composite(DeclarationType)
    ScalarLiteral(ScalarLiteral)
}
```

High level TypeField shape:

```ds
struct TypeField {
    key: StaticKey
    ty: LocalTypeId
    is_optional: bool
    is_readonly: bool
}
```

High level Instance shape:

```ds
struct Instance {
    symbol_id: GlobalSymbolId
    static_arguments: Vec<StaticArgument>
}
```

High level Resolution shape:

```ds
enum Resolution {
    Unresolved {
        receiver: Option<LocalTypeId>
        missing_keys: Vec<DispatchKey>
        candidates: Vec<ResolutionCandidate>
    }
    Builtin {
        receiver: Option<LocalTypeId>
    }
    Static {
        receiver: Option<LocalTypeId>
        candidate: ResolutionCandidate
    }
    Dynamic {
        receiver: Option<LocalTypeId>
        candidates: Vec<ResolutionCandidate>
    }
}

enum DispatchKey {
    Single { ty: LocalTypeId }
    Multiple { types: Vec<LocalTypeId> }
}

struct ResolutionCandidate {
    key: Option<DispatchKey>
    target_symbol: GlobalSymbolId
    instance: Option<LocalInstanceId>
}
```

Lower relies on Resolutions and Instances to avoid repeating overload and generic reasoning.

---

# Type Inference

Analyze uses bidirectional typing.
Inferred types flow out of expressions.
Expected types flow into expressions from context.
This is required for contextual typing of lambdas, object literals, and patterns.

**Sources of expected types**
* Variable and field annotations.
* Parameter types for call arguments.
* Return types for return expressions.
* Contextual types for lambdas and object literals.

Analyze uses inference variables and constraints for non trivial cases.
Inference variables are embedded in Type as placeholders for unknown types.
Constraints are collected during assignments, calls, returns, and pattern bindings.
A solver resolves inference variables to concrete Types or emits errors.

---

# Constraint Solving

Analyze collects constraints and then solves them in a deterministic order.
Constraints are normalized by expanding unions and intersections before solving.
Solver output is committed to the TypeTable once all constraints are processed.

**Constraint kinds**
* **Equality** constraints for explicit annotations and literal inference.
* **Subtype** constraints for assignability and argument to parameter flows.
* **Join** constraints for merged control flow environments.
* **Instantiation** constraints for static parameters and generic inference.
* **Conditional** constraints for flow based narrowing.
* **Candidate groups** for overload selection and ambiguity resolution.

**Solver order**
1. Collect constraints for a full expression tree or function body.
2. Normalize Types and expand unions and intersections.
3. Solve equality constraints first to reduce variable count.
4. Solve subtype constraints with variance aware propagation.
5. Solve candidate groups by best signature selection.
6. Choose defaults for remaining inference variables.
7. Commit final Types into the TypeTable and emit errors for conflicts.

---

# Flow Typing

Analyze performs flow sensitive typing using a control flow graph.
Each function has a CFG, and each block has a type environment.
Guard expressions narrow types on outgoing edges.
Merge points combine environments by union or a common supertype.
Unreachable paths are tracked for return, throw, break, and continue.

This behavior matches TypeScript narrowing semantics.
Pattern matching and match guards are part of this flow model.

High level CFG shape:

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

**Flow environment rules**
* A guard creates a refined environment for each outgoing edge.
* A merge combines environments per symbol.
* Unreachable blocks carry no environment and do not contribute to merges.

**Narrowing operators**

| Guard | True branch narrowing | False branch narrowing |
|-------|----------------------|------------------------|
| `x == null` | `x: null | undefined` | `x: Exclude<T, null | undefined>` |
| `x === null` | `x: null` | `x: Exclude<T, null>` |
| `typeof x == "string"` | `x: string` | `x: Exclude<T, string>` |
| `x instanceof C` | `x: C` | `x: Exclude<T, C>` |
| `"k" in x` | `x: T & { k: unknown }` | `x: Exclude<T, { k: unknown }>` |
| `x is T` | `x: T` | `x: Exclude<U, T>` |

---

# Resolutions

Analyze resolves member access and overloads at use sites.
Member access selects a member symbol from a Type, including Extensions.
Call and operator sites choose the best overload by assignability.
Union receivers may produce Dynamic Resolutions for runtime dispatch.

Resolutions are recorded in the TypeTable and never recomputed later.

**Overload selection order**
1. Filter candidates by arity and static parameter compatibility.
2. Infer static arguments from call site Types if needed.
3. Check assignability for each candidate in order of specificity.
4. Pick the most specific candidate or report ambiguity.
5. Record a Static or Dynamic Resolution at the call site node.

---

# Instances and Generics

Analyze creates Instances for statically parameterized declarations.
Static arguments can be explicit or inferred from usage.
The Instance stores a flattened list of static arguments for monomorphization.
This is required by Lower to produce specialized MIR.

**Instance creation rules**
* Inherited static arguments come first.
* Own static arguments come last.
* The Instance is stored on the node that introduced it.

---

# Type Operators and Compatibility

Analyze supports TypeScript style type operators and literal inference.
Type operators include keyof, typeof, indexed access, conditional types, and mapped types.
Type operators are represented as Type unary and binary operators or Unevaluated types.
Unevaluated types are resolved during Analyze or are rejected with loud errors.

TypeScript compatibility goals:
* Structural typing for objects and interfaces matches TypeScript.
* Contextual typing for lambdas and object literals matches TypeScript.
* Control flow narrowing matches TypeScript.
* Overload resolution follows TypeScript rules where possible.
* Literal inference and widening follow TypeScript rules by default.

Differences are explicit and limited:
* Ownership and mutability add new modifiers.
* Newtypes are nominal by default.
* Comptime requires explicit typing for non trivial cases.

---

# Tests and Coverage

The primary coverage lives in `language/test/fixtures/specification/`.
These fixtures validate inference and compatibility across most language features.
Unit tests in Analyze cover targeted edge cases and regressions.
