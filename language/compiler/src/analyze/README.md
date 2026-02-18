# Analyze

Analyze is where Destack commits program meaning on top of Resolve-identified symbol bindings.
Analyze commits declaration meaning, expression meaning, module boundary meaning, and semantic legality.
If a semantic fact is missing after Analyze, that is an Analyze bug.

## Pipeline

Analyze runs after Resolve and before Elaborate.

```text
Import -> Resolve -> Analyze -> Elaborate -> Execute -> Lower
```

Analyze is staged and ordered.

```text
Declare -> Export -> Infer -> Capture -> Validate
```

### Objectives

- Commit one coherent semantic meaning per declaration and expression.
- Preserve TypeScript-compatible behavior where Destack specifies compatibility.
- Keep cross-module semantic behavior deterministic and stage-gated.
- Make downstream phases consume semantic commitments instead of repairing them.

### Input

Analyze input is resolved DIR with symbol binding and module dependency structure (i.e. all the *Unresolved* variants should be gone and type as unknown in Analyze).

### Output

Analyze output is semantically committed DIR plus diagnostics.
Downstream phases should see stable declaration and expression meaning.

### Stage Model

| Stage | Primary question | Reads | Writes | Must not write |
| --- | --- | --- | --- | --- |
| Declare | What do declarations mean | Resolved symbols, declaration syntax | Declaration type facts and declaration-owned metadata | Expression flow and use-site inference facts |
| Export | What does this module promise to others | Resolve facts, Declare commitments | Export surface facts | Dependency internals and use-site expression facts |
| Infer | What does each expression mean here | Resolve facts, Declare commitments, Export commitments | Expression types, resolutions, flow facts, instance facts | Declaration-owned and export-owned facts |
| Capture | What closure environment data is required | Infer commitments | Capture metadata derived from infer facts | Declaration, export, infer, or validate semantic facts |
| Validate | Are committed semantics legal | Resolve facts, Declare commitments, Infer commitments | Diagnostics | Semantic type facts and resolution facts |

### Boundary Guarantees

| Boundary | Upstream guarantees | Analyze responsibility |
| --- | --- | --- |
| Resolve -> Analyze | Symbol targeting and module graph are known | Commit semantic meaning over those symbols |
| Analyze -> Elaborate | Declaration and expression semantics are committed | Emit complete semantic facts, not partial guesses |
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

- Evaluate declaration-side type expressions into canonical type facts.
- Commit declaration-owned associated member definitions, requirements, and defaults.
- Resolve declaration-context type-form ambiguities.
- Register declaration facts early enough for export and infer to consume without fallback.

### Declare Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolved declaration symbols and declaration syntax |
| output: declaration type facts | Canonical type meaning for declared declarations and members |
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

Infer consumes these declaration facts later.
Infer should not build these facts ad hoc at use sites.

### Declare Must Not

- Infer runtime expression flow or call-site behavior.
- Read later-stage inferred facts to patch missing declaration meaning.
- Emit fallback semantics that hide missing declaration ownership.

## Export

Export freezes the module boundary contract.
This stage decides what other modules may rely on without re-inferring this module internals.
Export is module-interface commitment, not expression checking.

### Export Responsibilities

- Commit exported symbol surface types from local committed facts.
- Apply legal local surface inference for exports.
- Publish boundary facts that dependency modules can read deterministically.

### Export Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolve facts plus Declare commitments in the local module |
| output | Stable exported type and symbol surface for dependency readers |

### Export Boundary Example

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

### Export Surface Rules

| Rule | Meaning |
| --- | --- |
| local surface inference is allowed | Exported shapes may be inferred from local committed syntax |
| dependency reinference is forbidden | Consumers do not derive provider internals from usage |
| boundary shape is stable | Cross-module reads depend on committed export surface |

### Export Must Not

- Infer dependency internals.
- Depend on fallback ordering to shape the boundary contract.
- Commit expression use-site semantics that belong to Infer.

## Infer

Infer commits expression meaning at use sites.
This stage is the checker core of Analyze.
Infer is where most semantic decisions become concrete program facts.

### Infer Responsibilities

- Contextual typing and expression type commitment.
- Resolution commitment for calls, members, operators, and overloads.
- Flow-sensitive narrowing and instance commitment.
- Static substitution and projection materialization at use sites.

### Infer Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolve facts plus Declare and Export commitments |
| output: expression type facts | Type meaning for expressions in context |
| output: resolution facts | Selected callable or member targets |
| output: flow facts | Narrowing and control-flow-refined types |
| output: instance and substitution facts | Concrete static argument instantiations and replacements |

### Infer Semantic Flow

Infer work is ordered so commitments are reproducible.

| Step | Operation | Result |
| --- | --- | --- |
| 1 | contextual typing | Expected types constrain expression analysis |
| 2 | candidate collection | Candidate members, calls, and overloads are gathered |
| 3 | relation and selection | Assignability and compatibility choose concrete targets |
| 4 | substitution and materialization | Static parameters and projection arguments become concrete |
| 5 | commitment | Type, resolution, flow, and instance facts are recorded |

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

`AuditLog.Segment` is committed using use-site substitution and projection materialization.
If required associated comptime projections are unresolved at a required concrete site, Infer emits a semantic error.

### Infer Behavior Example: Overload Commitment

Overload selection is committed at the use site, not deferred to downstream phases.

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

const a = parse("42");
const b = parse(42);
```

The two calls commit different selected overload targets.
Lower consumes those committed selections and does not rerun overload choice.

### Infer Must Not

- Rewrite declaration-owned metadata.
- Rewrite export-owned boundary contracts.
- Hide missing ownership facts behind generic fallback semantics.

## Validate

Validate checks legality over committed semantics.
Validate is semantically read-only.
Validate is the semantic policy and legality pass, not a semantic construction pass.

### Validate Responsibilities

- Enforce language legality rules over committed declaration and expression facts.
- Enforce profile policy checks over committed semantic facts.
- Emit diagnostics with accurate source context and committed semantic context.

### Validate Inputs And Outputs

| Item | Description |
| --- | --- |
| input | Resolve facts plus committed Declare and Infer semantic facts |
| output | Diagnostics only |

### Validate Read-Only Rule

Validate may inspect syntax to classify context.
Validate may read committed declaration and infer facts.
Validate must not create new semantic meaning as a side effect of checking.

### Validate Example

```ds
const state = { count: 0 };
state.count = 1;
```

If policy or contextual rules make this illegal in a given setting, Validate reports the legality error.
Validate does not fabricate new type facts to force the check to pass.

### Validate Rule Categories

| Category | Example question |
| --- | --- |
| language legality | Is this committed semantic usage legal in this language mode |
| profile policy | Is this usage legal under active profile restrictions |
| contextual diagnostics | What user-facing error best matches this committed semantic fact |

### Validate Must Not

- Evaluate expressions to create missing semantic type facts.
- Write inferred type facts or resolution facts.
- Repair ownership bugs from earlier stages.
