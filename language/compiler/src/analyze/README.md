# Analyze

Analyze turns resolved DIR into typed DIR with deterministic semantic metadata.
Analyze is where TypeScript compatibility and Destack semantics become concrete for each profile.
Elaborate, Execute, and Lower consume Analyze output and must not repair missing Analyze facts.

## Position in the Pipeline

Analyze runs after Resolve and before Elaborate.
Resolve owns symbol targeting, module graph construction, and export table construction.
Analyze owns typing, contextual inference, flow facts, and semantic diagnostics.
Elaborate owns canonicalization and reification and treats Analyze metadata as input.

```text
Import -> Resolve -> Analyze -> Elaborate -> Execute -> Lower
```

## Stage Model

Analyze is split into ordered module tasks per profile.
The stage order is declare, export, infer, capture, validate.
Each stage has strict write ownership and explicit read dependencies.

| Stage | Task | Owns |
| --- | --- | --- |
| Declare | `AnalyzeModuleDeclare` | Declaration type graph, associated type declarations, declaration metadata |
| Export | `AnalyzeModuleExport` | Public module boundary type surface |
| Infer | `AnalyzeModuleInfer` | Expression types, resolutions, contextual typing, instances, flow state |
| Capture | `AnalyzeModuleCapture` | Closure capture metadata |
| Validate | `AnalyzeModuleValidate` | Semantic diagnostics only |

## Core Terms

Analyze follows TypeScript terminology where possible because it makes edge cases debuggable.
The terms below are normative for code comments and implementation decisions.

- Contextual typing: an expected type influences an expression before widening.
- Freshness: object and array literals start fresh and lose freshness when they escape literal context.
- Widening: literal precision is relaxed at explicit commit points.
- Commit point: a stage writes stable facts that downstream stages may read but not rewrite.
- Materialization: static arguments are converted from syntax to stable type-level/static-level values.
- Substitution: static parameters are replaced with use-site arguments in a known substitution context.
- Evaluation: type expressions are reduced into canonical type ids.

## Stage Contracts

Stage ownership is a hard invariant.
Cross-stage fallback mutation is a bug.
Missing owner-stage data must fail loudly instead of being lazily created elsewhere.

### Declare

Declare builds declaration-owned structure.
Declare may evaluate declaration type syntax to establish canonical declaration metadata.
Declare owns associated type declaration maps, defaults, and constraints for class-shaped types.
Declare must not perform runtime expression inference.
Declare must not depend on remote inferred locals.

### Export

Export computes the module boundary type surface from local declaration data.
Export may use local surface inference for export declarations that can be derived from local syntax.
Export must not trigger remote module inference.
Export publishes boundary data that other modules can consume without re-inferring this module.

### Infer

Infer performs expression typing and flow-sensitive reasoning.
Infer owns contextual typing, freshness consumption, widening commit decisions, overload resolution, and instance registration.
Infer performs use-site substitution and materialization when concrete arguments are known.
Infer may project associated types through declaration-owned projection metadata using use-site substitutions.
Infer must not backfill missing declaration-owned metadata.

### Capture

Capture derives closure capture metadata from inferred state.
Capture must not mutate declaration-owned or infer-owned type state.

### Validate

Validate emits diagnostics against committed semantic state.
Validate must not materialize new types or mutate type graph data.
Validate may read infer results and emit policy-dependent severities.

## Mutation Matrix

The matrix below defines write ownership.
Any write outside the owner stage is a correctness bug.

| Data | Owner | Non-owner behavior |
| --- | --- | --- |
| Declared type ids and declaration alias targets | Declare | Read-only |
| Associated type declaration maps and defaults | Declare | Read-only |
| Export boundary type data | Export | Read-only |
| Inferred expression types and resolutions | Infer | Read-only |
| Instance table entries | Infer | Read-only |
| Capture metadata | Capture | Read-only |
| Diagnostics | Validate | Append-only |

## Public Boundary Rule

Analyze follows TypeScript boundary behavior with local surface inference.
Other modules consume exported boundary types and do not re-infer provider internals.
Cross-module inference cycles require explicit annotations to break recursion.

```ds
// a.ds
export const version = "v1";

// b.ds
import { version } from "./a";
version satisfies "v1";
```

## Contextual Typing, Freshness, and Widening

Contextual typing happens before widening.
Fresh literals are precise until they cross a commit boundary that consumes freshness.
Widening is explicit and owned by Infer.

```ds
const a = { mode: "dev" };
a.mode satisfies string;

const b = { mode: "dev" } as const;
b.mode satisfies "dev";
```

Infer owns this decision and Elaborate/Lower must not reinterpret it.

## Substitution, Materialization, and Evaluation

Substitution, materialization, and evaluation are distinct operations.
This separation is required for static parameters, associated type projections, and mapped or conditional operators.

### Placement Rules

- Declare may evaluate declaration-owned type syntax to establish stable metadata.
- Export may read declaration-owned evaluated forms but must not do use-site inference work.
- Infer performs use-site substitution and materialization where concrete arguments are known.
- Validate must not perform substitution or materialization to make checks pass.
- Pure relation checks consume existing type ids and must not trigger expression-to-type evaluation.

### Associated Type Projection Rules

Associated type declarations are declaration-owned metadata.
Associated type projection is a read-only lookup of declaration metadata plus use-site substitutions.
Projection lookup must not lazily hydrate declaration maps in Infer or Validate.
Interface defaults, inheritance, and sibling projections must use the same projection pipeline.

```ds
interface Container<T> {
    type Item;
}

struct Box<T> implements Container<T> {
    type Item = T;
}

const x: Box<int32>.Item = 1;
```

## Purity Boundary

Analyze separates pure relation logic from effectful evaluation logic.
This boundary is mandatory for determinism and for avoiding cross-stage feedback loops.

Pure logic includes assignability, comparability, identity, and relation-mode normalization over existing types.
Effectful logic includes declaration evaluation, static argument materialization, and require-gated remote metadata reads.

### Purity Rules

- `is_type_assignable*` and related relation helpers are read-only.
- Relation helpers do not call expression-to-type evaluation.
- Relation helpers do not perform default static argument resolution.
- Missing declaration-owned metadata in relation helpers is an invariant failure.

## Cross-Module Rules

Analyze is local by default and reads cross-module data through stage outputs.
Cross-module inference of dependency internals is forbidden.

### Boundary Rules

- Dependency reads for declaration-owned metadata require dependency declare completion.
- Dependency reads for boundary type data require dependency export completion.
- Dependency reads for infer-owned metadata require dependency infer completion.
- Cross-module reads must go through centralized require-gated access helpers.
- Cross-module cycles in boundary inference require explicit annotations.

## Flow and Narrowing Contract

Flow state is produced in Infer and consumed in Validate.
Validate may report diagnostics from flow state but does not compute new narrowings.
Guard semantics should match TypeScript for nullish checks, `typeof`, `instanceof`, `in`, and projection-friendly pattern checks.
Join behavior must be driven by CFG merge semantics, not ad hoc narrowing-vector equality.

```ds
if (value != null) {
    value satisfies NonNullable<typeof value>;
}
```

## Match and Pattern Contract

Infer owns pattern typing and arm result typing.
Validate owns exhaustiveness diagnostics and semantic pattern restrictions that require typed facts.
Match arm result types join into a union and then follow normal commit and contextual typing rules.
If the compiler cannot prove exhaustiveness, `_` is required.

```ds
const state = match (input) {
    0 => "zero"
    _ => "other"
};
```

## Relation Modes

Every relation call must choose an explicit relation mode.
Mode selection must be visible at call sites and propagated through nested checks.

| Mode | Purpose |
| --- | --- |
| Assignable | Assignment and argument compatibility |
| Comparable | Equality and comparison compatibility |
| Identity | Exact semantic type identity |
| TypeOps | Type-operator semantics (`keyof`, mapped, conditional, indexed access) |

## Option and Policy Routing

Compiler options are part of the language contract and must be stage-stable.
Checks that need inferred facts belong in Infer or Validate based on data ownership.
Pure syntax restrictions belong in Parse.
Analyze emits one diagnostic code path and maps policy severity centrally for `allow`, `warn`, and `deny`.

## Analyze and Elaborate Boundary

Elaborate is downstream and separate.
Elaborate may query Analyze metadata and rewrite IR accordingly.
Elaborate must not mutate Analyze-owned semantic state.
Missing required Analyze metadata during Elaborate is a compiler invariant violation.

## Analyze and Lower Boundary

Lower expects fully typed canonical DIR where substitutions and projections are already resolved.
Lower does not perform type inference and does not repair missing Analyze metadata.
Lower may consume resolution decisions and runtime-check planning metadata, but it does not re-type expressions.

## Concurrency and Locking

Analyze runs in parallel workers.
Correctness depends on deterministic require-gate ordering and lock discipline.

- Keep lock scopes narrow.
- Avoid nested lock stacks that can re-enter stage work while locks are held.
- Prefer importing remote snapshots into local temporaries over long-lived remote locks.

## Module Map

The ownership map below is the expected structure.

| Concern | Primary modules |
| --- | --- |
| Stage orchestration | `analyze/process.rs`, `analyze/*/process.rs` |
| Declaration-owned evaluation | `analyze/declare/*` |
| Boundary type computation | `analyze/export/*` |
| Inference and relations | `analyze/infer/*` |
| Shared helpers | `analyze/common/*` |
| Semantic validation | `analyze/validate/*` |
| Capture metadata | `analyze/capture/*` |

`analyze/common/*` should keep pure helpers and effectful helpers clearly separated.
Effectful helpers should be explicit in naming and stage-safe by caller contract.

## Failure Policy

Analyze fails loudly on invariant violations.
Silent recovery through cross-stage mutation is forbidden.
If required owner-stage metadata is missing, emit an explicit compiler error and keep ownership boundaries intact.

## Test Surface

Analyze correctness is validated by specification fixtures, targeted compiler tests, and conformance suites.
Use these as the primary guardrails during refactors.

```bash
cargo test --release -p destack_compiler
cargo test --release -p destack_test --test specification
```

Contract violations should be fixed in stage ownership and helper architecture, not patched locally.
