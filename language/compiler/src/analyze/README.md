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
Analyze computes Types and Resolutions based on those symbols.
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
TypeTable is module local and not thread safe by design.
Elaborate and Lower rely on these results and do not recompute types or resolve overloads again.

High level TypeTable shape:

```ds
struct TypeTable {
    moduleId: ModuleId

    nextTypeId: u32
    types: Arena<Type>
    sourceIdByTypeId: LocalNodeIdAny[]

    declaredTypeByNodeId: Map<GlobalNodeIdAny, LocalTypeId>
    inferredTypeByNodeId: Map<GlobalNodeIdAny, LocalTypeId>

    instanceTypeBySymbolId: Map<GlobalSymbolId, LocalTypeId>
    valueTypeBySymbolId: Map<GlobalSymbolId, LocalTypeId>

    nextInstanceId: u32
    instances: Arena<Instance>
    instanceByNodeId: Map<GlobalNodeIdAny, LocalInstanceId>

    nextResolutionId: u32
    resolutions: Arena<Resolution>
    resolutionByNodeId: Map<GlobalNodeIdAny, LocalResolutionId>

    nextLineageId: u32
    lineages: Arena<Lineage>
    lineageBySymbolId: Map<GlobalSymbolId, LocalLineageId>

    nextExtensionId: u32
    extensions: Arena<Extension>
    extensionBySymbol: Map<GlobalSymbolId, LocalExtensionId>
    extensionsByTarget: Map<GlobalSymbolId, LocalExtensionId[]>
}
```

Each node can have a **declared Type**, an **inferred Type**, or both.
Each symbol can have an **instance Type** and a **value Type**.
Instances and Resolutions are recorded at the node that triggered them.

**Key invariants**
1. Declared Types come only from explicit annotations.
2. Inferred Types are computed for all reachable expressions.
3. Instance Types describe the shape of a declared type.
4. Value Types describe the type a symbol has at use sites.
5. Resolutions capture overload selection and are stable across phases.

Analyze also maintains an InferTable during analysis.
InferTable stores inference variables, constraints, and solver state for a module.
InferTable is resolved before Elaborate and is not consumed by later passes.

High level InferTable shape:

```ds
struct InferTable {
    vars: InferVar[]
    constraints: Constraint[]
    varByNodeId: Map<GlobalNodeIdAny, InferVarId>
    varBySymbolId: Map<GlobalSymbolId, InferVarId>
    varByTypeParameter: Map<GlobalSymbolId, InferVarId>
    typeByVarId: LocalTypeId[]
}

struct InferVar {
    lowerBounds: LocalTypeId[]
    upperBounds: LocalTypeId[]
    defaultType: LocalTypeId | undefined
    origin: InferOrigin
    scope: InferScope
}

newtype InferOrigin =
    | { kind: "Expression" = "Expression", node: GlobalNodeIdAny }
    | { kind: "Parameter" = "Parameter", node: GlobalNodeIdAny }
    | { kind: "Return" = "Return", node: GlobalNodeIdAny }
    | { kind: "TypeParameter" = "TypeParameter", symbol: GlobalSymbolId }
    | { kind: "ConstraintGroup" = "ConstraintGroup", groupId: ConstraintGroupId }

struct InferScope {
    owner: GlobalSymbolId
    functionId: GlobalNodeIdAny | undefined
}

newtype Constraint =
    | { kind: "Equal" = "Equal", left: LocalTypeId, right: LocalTypeId }
    | {
          kind: "Subtype" = "Subtype"
          sub: LocalTypeId
          sup: LocalTypeId
          variance: VarianceBound | undefined
      }
    | { kind: "Join" = "Join", target: InferVarId, sources: LocalTypeId[] }
    | {
          kind: "Instantiate" = "Instantiate"
          target: InferVarId
          generic: LocalTypeId
          args: LocalTypeId[]
      }
    | {
          kind: "Conditional" = "Conditional"
          guard: LocalTypeId
          whenTrue: LocalTypeId
          whenFalse: LocalTypeId
      }
    | {
          kind: "CandidateGroup" = "CandidateGroup"
          id: ConstraintGroupId
          options: Constraint[][]
      }
```

InferContext carries transient state during inference.
InferContext is created and forked as the analyzer descends the DIR tree.

High level InferContext shape:

```ds
struct InferContext {
    narrowings: [GlobalSymbolId, LocalTypeId][]
    expectedType: LocalTypeId | undefined
    returnType: LocalTypeId | undefined
    isUnreachable: boolean
    inLoop: LocalNodeIdAny | undefined
    inMatch: LocalNodeIdAny | undefined
    inFunction: LocalNodeIdAny | undefined
    isAsync: boolean
    isGenerator: boolean
    inAbstractClass: boolean
}
```

---

# Type Representation

Analyze operates on Type, Instance, Resolution, Lineage, and Extension structures that are stable across passes.
Type records the semantic shape of values and types in DIR.
Instance records a concrete instantiation of a statically parameterized declaration.
Resolution records member selection and overload dispatch results.
Lineage records resolved inheritance for nominal types.
Extension records resolved extension declarations and visibility.

High level Type shape:

```ds
newtype Type =
    | { kind: "TypeLiteral" = "TypeLiteral", value: TypeLiteral }
    | { kind: "InferVar" = "InferVar", id: InferVarId }
    | { kind: "Value" = "Value", value: LocalTypeId }
    | {
          kind: "Reference" = "Reference"
          symbol: GlobalSymbolId
          staticArguments: StaticArgument[] | undefined
      }
    | { kind: "Unevaluated" = "Unevaluated", node: LocalNodeId<Expression> }

    | { kind: "Unary" = "Unary", operator: TypeUnaryOperator, right: LocalTypeId }
    | { kind: "Mutable" = "Mutable", mutability: Mutability, right: LocalTypeId }
    | {
          kind: "ValueOf" = "ValueOf"
          mutability: Mutability | undefined
          variance: VarianceBound | undefined
          right: LocalTypeId
      }
    | {
          kind: "ReferenceOf" = "ReferenceOf"
          mutability: Mutability | undefined
          variance: VarianceBound | undefined
          right: LocalTypeId
      }
    | {
          kind: "Binary" = "Binary"
          left: LocalTypeId
          operator: TypeBinaryOperator
          right: LocalTypeId
      }

    | {
          kind: "ArraySized" = "ArraySized"
          element: LocalTypeId
          count: LocalNodeId<Expression>
      }
    | { kind: "Array" = "Array", element: LocalTypeId | undefined }
    | { kind: "Tuple" = "Tuple", elements: LocalTypeId[] }
    | { kind: "Object" = "Object", fields: TypeField[] }
    | {
          kind: "Function" = "Function"
          asynchrony: Asynchrony
          cardinality: FunctionCardinality
          staticParameters: LocalTypeId[]
          dynamicParameters: LocalTypeId[]
          returnType: LocalTypeId | undefined
      }

    | { kind: "Union" = "Union", elements: LocalTypeId[] }
    | { kind: "Intersection" = "Intersection", elements: LocalTypeId[] }

    | { kind: "Error" = "Error" }
```

High level TypeLiteral shape:

```ds
newtype TypeLiteral =
    | { kind: "Never" = "Never" }
    | { kind: "Any" = "Any" }
    | { kind: "Infer" = "Infer" }
    | { kind: "Undefined" = "Undefined" }
    | { kind: "Unknown" = "Unknown" }
    | { kind: "Void" = "Void" }
    | { kind: "Null" = "Null" }
    | { kind: "Primitive" = "Primitive", value: PrimitiveType }
    | { kind: "Composite" = "Composite", value: DeclarationType }
    | { kind: "ScalarLiteral" = "ScalarLiteral", value: ScalarLiteral }
```

High level TypeField shape:

```ds
struct TypeField {
    key: StaticKey
    ty: LocalTypeId
    isOptional: boolean
    isReadonly: boolean
}
```

High level TypeKind shape:

```ds
newtype TypeKind =
    | { kind: "Structural" = "Structural" }
    | { kind: "Nominal" = "Nominal" }
```

High level Instance shape:

```ds
struct Instance {
    symbolId: GlobalSymbolId
    staticArguments: StaticArgument[]
}
```

High level StaticParameterKind shape:

```ds
newtype StaticParameterKind =
    | { kind: "Type" = "Type" }
    | { kind: "Value" = "Value" }
```

High level StaticParameter shape:

```ds
struct StaticParameter {
    symbol: GlobalSymbolId
    name: StringId | undefined
    declaredTypeId: LocalTypeId
    defaultExpression: GlobalNodeId<Expression> | undefined
}
```

High level StaticArgument shape:

```ds
newtype StaticArgument =
    | { kind: "Unevaluated" = "Unevaluated", node: LocalNodeId<Argument> }
    | {
          kind: "Evaluated" = "Evaluated"
          name: StringId | undefined
          value: StaticExpression
      }
```

High level StaticExpression shape:

```ds
newtype StaticExpression =
    | { kind: "Unevaluated" = "Unevaluated", node: LocalNodeId<Expression> }
    | { kind: "ScalarLiteral" = "ScalarLiteral", value: ScalarLiteral }
    | { kind: "TypeLiteral" = "TypeLiteral", value: TypeLiteral }
    | {
          kind: "Declaration" = "Declaration"
          declaration: LocalNodeId<Declaration>
          staticArguments: StaticArgument[] | undefined
      }
    | { kind: "Type" = "Type", ty: LocalTypeId }
    | {
          kind: "RangeExpression" = "RangeExpression"
          start: StaticExpression
          end: StaticExpression
          isInclusive: boolean
      }
    | { kind: "ArrayExpression" = "ArrayExpression", elements: StaticExpression[] }
    | { kind: "TupleExpression" = "TupleExpression", elements: StaticExpression[] }
    | { kind: "ObjectExpression" = "ObjectExpression", properties: StaticProperty[] }
```

High level Resolution shape:

```ds
newtype Resolution =
    | {
          kind: "Unresolved" = "Unresolved"
          receiver: LocalTypeId | undefined
          missingKeys: DispatchKey[]
          candidates: ResolutionCandidate[]
      }
    | { kind: "Builtin" = "Builtin", receiver: LocalTypeId | undefined }
    | {
          kind: "Static" = "Static"
          receiver: LocalTypeId | undefined
          candidate: ResolutionCandidate
      }
    | {
          kind: "Dynamic" = "Dynamic"
          receiver: LocalTypeId | undefined
          candidates: ResolutionCandidate[]
      }

newtype DispatchKey =
    | { kind: "Single" = "Single", ty: LocalTypeId }
    | { kind: "Multiple" = "Multiple", types: LocalTypeId[] }

struct ResolutionCandidate {
    key: DispatchKey | undefined
    targetSymbol: GlobalSymbolId
    instance: LocalInstanceId | undefined
}
```

High level Lineage shape:

```ds
struct Lineage {
    extends: GlobalSymbolId | undefined
    implements: GlobalSymbolId[]
    embedded: GlobalSymbolId[]
}
```

High level Extension shape:

```ds
newtype ExtensionKind =
    | { kind: "Inherent" = "Inherent" }
    | { kind: "Local" = "Local" }
    | { kind: "Nominal" = "Nominal" }

struct Extension {
    symbol: GlobalSymbolId
    kind: ExtensionKind
    target: GlobalSymbolId
    lineage: LocalLineageId | undefined
}
```

---

# Type Inference

Analyze uses bidirectional typing.
Inferred types flow out of expressions.
Expected types flow into expressions from context.
This is required for contextual typing of lambdas, object literals, and patterns.

**Sources of expected types**
1. Variable and field annotations.
2. Parameter types for call arguments.
3. Return types for return expressions.
4. Contextual types for lambdas and object literals.

Inference variables are embedded in Type as placeholders for unknown types.
Constraints are collected during assignments, calls, returns, and pattern bindings.
The solver resolves inference variables to concrete Types or emits errors.

InferContext supplies expected types and return types to nested expressions.
InferContext also stores reachability state for error reporting.

---

# Constraint Solving

Analyze collects constraints and then solves them in a deterministic order.
Constraints are normalized by expanding unions and intersections before solving.
Solver output is committed to the TypeTable once all constraints are processed.

**Constraint kinds**
1. Equality constraints for explicit annotations and literal inference.
2. Subtype constraints for assignability and argument to parameter flows.
3. Join constraints for merged control flow environments.
4. Instantiation constraints for static parameters and generic inference.
5. Conditional constraints for flow based narrowing.
6. Candidate groups for overload selection and ambiguity resolution.

**Solver order**
1. Collect constraints for a full expression tree or function body.
2. Normalize Types and expand unions and intersections.
3. Solve equality constraints first to reduce variable count.
4. Solve subtype constraints with variance aware propagation.
5. Solve candidate groups by best signature selection.
6. Choose defaults for remaining inference variables.
7. Commit final Types into the TypeTable and emit errors for conflicts.

---

# Where Clauses

Where clauses are lowered into constraints during Analyze.
Each clause contributes subtype and equality constraints on static parameters and inference variables.

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
1. A guard creates a refined environment for each outgoing edge.
2. A merge combines environments per symbol.
3. Unreachable blocks carry no environment and do not contribute to merges.

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

# Flow Data Reuse

Flow typing results are recorded so later passes can query narrowed types.
This allows the linter and other analysis steps to reuse flow state without recomputation.
The FlowTable is introduced together with the CFG implementation.

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
Static arguments can appear on member expressions or on call expressions.
If both are present, Analyze reports a ConflictingStaticArguments error.
Call static arguments drive inference for that call.
Member static arguments stay attached to the member expression Instance.
The Instance stores a flattened list of static arguments for monomorphization.
This is required by Lower to produce specialized MIR.

**Instance creation rules**
1. Inherited static arguments come first.
2. Own static arguments come last.
3. The Instance is stored on the node that introduced it.

---

# Type Operators and Compatibility

Analyze supports TypeScript style type operators and literal inference.
Type operators include keyof, typeof, indexed access, conditional types, and mapped types.
Type operators are represented as Type unary and binary operators or Unevaluated types.
Unevaluated types are resolved during Analyze or are rejected with loud errors.

TypeScript compatibility goals:
1. Structural typing for objects and interfaces matches TypeScript.
2. Contextual typing for lambdas and object literals matches TypeScript.
3. Control flow narrowing matches TypeScript.
4. Overload resolution follows TypeScript rules where possible.
5. Literal inference and widening follow TypeScript rules by default.

Differences are explicit and limited:
1. Ownership and mutability add new modifiers.
2. Newtypes are nominal by default.
3. Comptime requires explicit typing for nontrivial cases.

---

# Builtin Types

Intrinsic types like Readonly, Pick, and Record are modeled as builtin operators in Analyze.
Library types ("canonical symbols") like Promise and Iterator are provided by core modules and mapped with LanguageItem identifiers.
This keeps TypeScript compatibility while preserving explicit module ownership.

---

# Module Boundaries

Analyze keeps a TypeTable per module.
When an expression references a symbol from another module, Analyze copies the referenced Type into the local TypeTable.
Structural types are copied recursively, and nominal references retain their GlobalSymbolId identity.
InferVar values from remote modules are replaced with Unknown to keep inference local.
Exports should have declared Types for stable cross module typing.

---

# Implementation Status // nocheckin: remove status when done

Analyze is implemented in layers that match the pipeline contract.
The current implementation covers the following.

1. Type evaluation for function declarations, object types, tuple types, array types, and literal types.
   Declared Types are recorded for annotated nodes.
2. Expression inference for literals, arrays, tuples, objects, calls, and lambdas.
   Contextual typing drives lambda parameter types and literal widening.
3. Instance creation for function references, member access, and call sites with static arguments.
   Static argument inference uses the InferTable and is recorded on the originating node.
4. Assignability checks for literal types, unions, arrays, tuples, and objects.
   Errors are recorded without aborting the analysis pass.

The current implementation does not yet cover the following.

1. Resolution and overload selection at call and member sites.
   Resolution records are not populated yet.
2. CFG based flow typing with branch sensitive environments.
   InferContext stores narrowings but no CFG is constructed.
3. Where clause lowering into constraint groups.
   Where clauses are parsed but are not applied during Analyze yet.
4. Full declaration level evaluation for classes, enums, interfaces, and module scope statements.
   Only function declaration evaluation is implemented now.

---

# Tests and Coverage

The primary coverage lives in `language/test/fixtures/specification/`.
These fixtures validate inference and compatibility across language features.
Unit tests in Analyze cover targeted edge cases and regressions.
Run the compiler test suite with `cargo test --release -p destack_compiler`.
Run the specification suite with `cargo test --release -p destack_test --test specification`.
