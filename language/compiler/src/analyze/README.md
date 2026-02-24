# Analyze

Analyze is where Destack commits program meaning on top of Resolve-identified symbol bindings.
Analyze commits declaration meaning, expression meaning, module boundary meaning, and semantic legality.
If a semantic commitment is missing after Analyze, that is an Analyze bug.

## Pipeline

Analyze runs after Resolve and before Elaborate.

```text
Import -> Resolve -> Analyze -> Elaborate -> Execute -> Lower
```

Analyze is staged and ordered.

```text
Declare -> Interface -> Infer -> Solve -> Commit -> Validate
```

### Objectives

- Commit one coherent semantic meaning per declaration and expression.
- Preserve TypeScript-compatible behavior where Destack specifies compatibility.
- Keep cross-module semantic behavior deterministic and stage-gated.
- Make downstream phases consume semantic commitments instead of repairing them.

### Input

Analyze input is resolved DIR with symbol bindings and module dependency structure.
All `Unresolved*` forms should already be eliminated before Analyze.
`unknown` remains a real type value inside Analyze.

### Output

Analyze output is semantically committed DIR plus diagnostics.
Downstream phases should see stable declaration and expression meaning.

### Stage Model

| Stage | Primary question | Reads | Writes | Must not write |
| --- | --- | --- | --- | --- |
| Declare | What do declarations mean | Resolved symbols, declaration syntax | Declaration type commitments and declaration-owned metadata | Expression flow and use-site inference records |
| Interface | What does this module promise to others | Resolve outputs, Declare commitments | Interface surface commitments | Dependency internals and use-site expression records |
| Infer | What provisional meaning is implied by local usage | Resolve outputs, Declare commitments, Interface commitments | Provisional expression types, constraints, obligations, and flow refinements | Canonical semantic commitments |
| Solve | Which provisional records survive global convergence | Infer provisional records | Solved substitutions and canonicalized inference outcomes | Canonical semantic commitments and diagnostics |
| Commit | Which solved records become canonical program meaning | Solve outputs and committed upstream commitments | Committed expression types, resolutions, instances, and solved overlays | New inference constraints and diagnostics |
| Validate | Are committed semantics legal | Resolve outputs, Declare commitments, Interface commitments, Commit commitments | Diagnostics | Semantic type commitments and resolution commitments |

### Implementation scheduling note

Current task scheduling runs a concrete `Capture` task after commit and before validate.
`Capture` is a derived metadata pass over committed semantics.
It is not a semantic phase in the canonical ownership lifecycle.

### Boundary Guarantees

| Boundary | Upstream guarantees | Analyze responsibility |
| --- | --- | --- |
| Resolve -> Analyze | Symbol targeting and module graph are known | Commit semantic meaning over those symbols |
| Analyze -> Elaborate | Declaration and expression semantics are committed | Emit complete semantic commitments, not partial guesses |
| Analyze -> Execute | Static and inferred meaning is committed | Avoid late semantic fabrication in Execute |
| Analyze -> Lower | Types and resolutions are coherent | Keep type or resolution holes out of codegen input |

### Downstream Contract

Elaborate, Execute, and Lower consume Analyze commitments as fixed semantic input.
They may lower and reify semantics, but they must not fabricate missing semantic meaning.

## Declare

Declare gives declarations their semantic shape.
This is where declaration-side type expressions are evaluated and declaration-owned metadata is fixed.
Declare is declaration-first, not use-site-first.

### Declare Responsibilities

- Evaluate declaration-side type expressions into canonical type commitments.
- Commit declaration-owned associated member definitions, requirements, and defaults.
- Report missing associated type and associated comptime requirements while declaration contracts are collected.
- Resolve declaration-context type-form ambiguities.
- Register declaration commitments early enough for interface and infer to consume without fallback.

### Declare Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolved declaration symbols and declaration syntax |
| output: declaration type commitments | Canonical type meaning for declared declarations and members |
| output: declaration associated metadata | Required and default associated type or comptime members |
| output: declaration disambiguation outcomes | Chosen interpretation for declaration-context ambiguous type forms |

### Declare Disambiguation Example

Declare owns declaration-context interpretation of `T[K]` forms.
The default is TypeScript indexed access when the index is type-space.
`as comptime` explicitly forces fixed-array interpretation in value-space.

```ds
type Field<T, K: keyof T> = T[K];

type Row<T, comptime N: number> = T[N as comptime];
```

The first declaration is indexed access.
The second declaration is fixed-array construction.

### Declare Associated Metadata Example

Declare commits declaration-owned associated contracts before use-site inference.

```ds
interface Container<T> {
    type Item;
    comptime const Capacity: number = 16;
}

class StringBox implements Container<string> {
    type Item = string;
    comptime const Capacity: number = 32;
}
```

Infer consumes these declaration commitments later.
Infer should not build these commitments ad hoc at use sites.

### Declare Diagnostic Ownership For Associated Requirements

Missing associated requirements are declaration-shape diagnostics.
Declare owns these checks so cross-module dependency analysis cannot skip them when Validate is not scheduled for dependency reads.
Validate may still report legality over committed semantics, but it does not own this declaration contract completeness check.

### Declare Must Not

- Infer runtime expression flow or call-site behavior.
- Read later-stage inferred records to patch missing declaration meaning.
- Emit fallback semantics that hide missing declaration ownership.

## Interface

Interface freezes the module boundary contract.
This stage decides what other modules may rely on without re-inferring this module internals.
Interface is module-interface commitment, not expression checking.

### Interface Responsibilities

- Commit exported symbol surface types from local commitments.
- Apply legal local surface inference for exports.
- Publish boundary commitments that dependency modules can read deterministically.

### Interface Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolve outputs plus Declare commitments in the local module |
| output | Stable exported type and symbol surface for dependency readers |

### Interface Boundary Example

Exports may use local surface inference.
Consumers read the committed export surface rather than re-running local inference logic.

```ds
// a.ds
export const version = "v1";
export function add(a: int32, b: int32) { a + b }

// b.ds
import { version, add } from "./a";
```

Module `b` reads the committed boundary shape from module `a`.
Module `b` does not re-infer module `a` internals.

### Interface Surface Rules

| Rule | Meaning |
| --- | --- |
| local surface inference is allowed | Exported shapes may be inferred from local committed syntax |
| dependency reinference is forbidden | Consumers do not derive provider internals from usage |
| boundary shape is stable | Cross-module reads depend on committed export surface |

### Cross-Module Barrier Rule

`Interface` is the only cross-module semantic dependency barrier in Analyze.
Dependency modules may require `Declare` and `Interface`.
Dependency modules must not require `Infer`, `Solve`, `Commit`, or `Validate`.

### Interface Publication Domains

Cross-module value reads use one explicit publication-domain split.
This split prevents stage leakage while still allowing associated member semantics.

| Symbol kind | Read domain | Owner stage |
| --- | --- | --- |
| module interface value symbols (published exports) | committed interface value type commitments | Interface |
| non-export value symbols reachable through published references (for example associated or class members) | declaration-backed symbol commitments | Declare |

Infer must not guess between these domains.
Infer selects the domain from symbol publication status and reads through stage-gated queries only.

### Interface Must Not

- Infer dependency internals.
- Depend on fallback ordering to shape the boundary contract.
- Commit expression use-site semantics that belong to Commit.

## Infer

Infer models expression meaning at use sites.
This stage is the checker core of Analyze.
Infer is where most semantic decisions become provisional semantic records.

### Infer Responsibilities

- Contextual typing and expression typing.
- Resolution candidate collection for calls, members, operators, and overloads.
- Flow-sensitive narrowing and constraint generation.
- Obligation generation for diagnostics that depend on solved substitutions.
- Provisional substitution and projection materialization inputs for Solve.

### Infer Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolve outputs plus Declare and Interface commitments |
| output: provisional expression records | Candidate type meaning for expressions in context |
| output: provisional resolution records | Candidate callable or member targets and relation details |
| output: infer constraints | Subtype, equality, and relation constraints for solver convergence |
| output: infer obligations | Deferred diagnostics and post-solve validation obligations |
| output: provisional flow records | Narrowing and control-flow-refined type candidates |

### Obligation Lifecycle

Infer records obligations when correctness depends on solved substitutions or solved receiver state.
Solve resolves placeholder operands and canonicalizes substitutions used by those obligations.
Commit discharges obligations in deterministic order and emits diagnostics when checks fail.
Validate is read-only over committed state and does not mutate obligation state.

### Instantiation And Resolution Record Contract

Infer records instantiation and dispatch outcomes as separate but linked provisional record families.
An `Instance` is canonical semantic identity for one concrete static substitution environment.
The canonical identity key is `(origin symbol, normalized substitution environment)`.
The `instance_by_node_id` table is a use-site index only.
Node placement does not own instance identity.
Resolution ownership stays in `resolution_by_node_id`.

Infer records one `Instantiation Event` whenever a node materializes concrete static substitutions for one symbol.
Each event records provisional node linkage and unresolved or solved substitution state for Solve.
Unevaluated or unresolved static arguments must not commit instances.

Infer records static and dynamic dispatch outcomes through provisional resolution records.
Static single-target coherence is checked after Solve and enforced in Commit.
Dynamic resolution may carry per-candidate provisional instances without requiring node-level instance commitment.
Type-space instantiation sites may record provisional instances without runtime resolution records.

### Instance Placement Rules

The placement rules below are hard invariants for Analyze correctness and lowering soundness.

| Node category | Commit instance on node | Rule |
| --- | --- | --- |
| Type reference expression | yes, when resolved static arguments are non-empty and evaluated | type instantiation event |
| Call expression | yes, when callee or member static substitutions are non-empty and evaluated | invocation instantiation event |
| New expression | yes, when constructor static substitutions are non-empty and evaluated | constructor instantiation event |
| Member expression used as value with static arguments | yes, when static substitutions are non-empty and evaluated | member-value instantiation event |
| Declarations and declarators and patterns and statements | no | not instantiation events |
| Plain references and non-generic calls | no | no concrete static substitution environment |

Lower and later phases must consume committed instance ids only.
Lower must not synthesize instance identities from syntax.
Any missing instance commitment at a required monomorphization site is an Analyze bug.

### Infer Semantic Flow

Infer work is ordered so commitments are reproducible.

| Step | Operation | Result |
| --- | --- | --- |
| 1 | contextual typing | Expected types constrain expression analysis |
| 2 | candidate collection | Candidate members, calls, and overloads are gathered |
| 3 | relation and selection | Assignability and compatibility choose concrete targets |
| 4 | substitution and materialization capture | Static parameters and projection inputs are captured for solving |
| 5 | provisional record write | Constraints, obligations, and provisional inference records are recorded |

### Infer Behavior Example: Contextual Typing And Widening

Infer applies contextual typing before widening commitments.

```ds
const a = { mode: "dev" };

const b = { mode: "dev" } as const;
```

`a.mode` widens to `string` under normal object literal rules.
`b.mode` remains the literal `"dev"` because the const context suppresses widening.

### Infer Behavior Example: Associated Projection At Use Site

Infer resolves and materializes associated projections where expressions need concrete meaning.

```ds
interface LogStore<Record> {
    comptime const SegmentRows: number = 1024;
    type Segment = Record[this.SegmentRows];
}

class AuditLog implements LogStore<string> {
    comptime const SegmentRows: number = 2048;
}

const row: AuditLog.Segment = "ok";
```

`AuditLog.Segment` is solved and committed using use-site substitution and projection materialization.
If required associated comptime projections are unresolved at a required concrete site, the post-solve obligation is reported in Validate.

### Infer Behavior Example: Overload Commitment

Overload selection records are collected at the use site and committed after solve convergence.

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

const a = parse("42");
const b = parse(42);
```

The two calls produce distinct solved overload targets.
Commit writes those canonical selections.
Lower consumes those committed selections and does not rerun overload choice.

### Infer Must Not

- Rewrite declaration-owned metadata.
- Rewrite interface-owned boundary contracts.
- Hide missing ownership commitments behind generic fallback semantics.
- Commit canonical semantic commitments to shared program tables.
- Emit diagnostics that require solved substitution state.

## Solve

Solve converges provisional infer state into canonical solved state.
Solve is deterministic and semantically read-write only over infer-owned provisional state.

### Solve Responsibilities

- Solve inference variables and relation constraints.
- Run fixed-point convergence for interface-sensitive and projection-sensitive equations.
- Canonicalize substitutions and zonk provisional type references.
- Resolve deferred obligation operands against solved state.
- Prepare canonical commit state for Commit.

### Solve Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Infer provisional constraints, obligations, and records |
| output: solved substitutions | Canonical static and type substitutions |
| output: solved records | Commit-ready resolution and instantiation state |
| output: solved obligations | Deferred diagnostics with solved operand state |

### Solve Must Not

- Write canonical shared semantic commitments to program tables.
- Emit user diagnostics directly.
- Reopen declaration or interface ownership decisions.

## Commit

Commit writes solved semantic commitments into canonical shared tables.
Commit is the only stage allowed to mutate committed expression and resolution semantics.

### Commit Responsibilities

- Commit solved expression type commitments.
- Commit solved resolution commitments for calls, members, operators, and overload selections.
- Intern canonical instances and attach canonical `InstanceId` references to nodes and resolution candidates.
- Commit solved symbol-value overlays that are infer-owned.
- Enforce commit-time coherence invariants between resolution and instance commitments.

### Commit Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Solve outputs plus committed Declare and Interface commitments |
| output | Canonical shared semantic commitments for downstream stages |

### Commit Must Not

- Perform new inference or add new constraints.
- Perform convergence or fixed-point solving.
- Emit diagnostics.

## Validate

Validate checks legality over committed semantics.
Validate is semantically read-only.
Validate is the semantic policy and legality pass, not a semantic construction pass.

### Validate Responsibilities

- Enforce language legality rules over committed declaration and expression commitments.
- Enforce profile policy checks over committed semantic commitments.
- Emit diagnostics with accurate source context and committed semantic context.

### Validate Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolve outputs plus committed Declare, Interface, and Commit semantic commitments |
| output | Diagnostics only |

### Validate Read-Only Rule

Validate may inspect syntax to classify context.
Validate may read committed declaration and infer commitments.
Validate must not create new semantic meaning as a side effect of checking.

### Validate Example

```ds
const state = { count: 0 };
state.count = 1;
```

If policy or contextual rules make this illegal in a given setting, Validate reports the legality error.
Validate does not fabricate new type commitments to force the check to pass.

### Validate Rule Categories

| Category | Example question |
| --- | --- |
| language legality | Is this committed semantic usage legal in this language mode |
| profile policy | Is this usage legal under active profile restrictions |
| contextual diagnostics | What user-facing error best matches this committed semantic state |

### Validate Must Not

- Evaluate expressions to create missing semantic type commitments.
- Write inferred type commitments or resolution commitments.
- Repair ownership bugs from earlier stages.
