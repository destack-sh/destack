# Analyze Refactor Plan

This plan defines the final, systematic shape of Analyze with explicit contracts, minimal duplication, and predictable cross-module behavior.
It targets long-term correctness and performance rather than incremental band-aids.
We aim to remove and replace Analyze code as much as possible while being correct.
Refactoring is good and required.
As we discover useful abstractions or refactor helpers, update this plan immediately with the new details and placement guidance.
When we discover new systematic issues or stopgaps, add them to the appropriate track and note whether they are temporary.

## Goals

This section states the end-state properties we want to guarantee.

- Declare, export, and infer have crisp responsibilities and do not reach across phase boundaries in ad-hoc ways.
- Cross-module reads must go through explicit require gates, and never by directly poking remote tables.
- Static parameter constraints are represented symbolically during shape construction, and only validated at explicit validation sites.
- Type traversal and type rewriting use shared walkers rather than bespoke recursive functions sprinkled across the codebase.
- Normalization caching is sound by construction and invalidated narrowly, not via a global epoch bump after every mutation.
- Tests primarily live in the spec suite, and unit tests only cover internal invariants that the spec cannot express.

## Refactor First Policy

This plan is explicitly refactor-first and contract-first.
We are not trying to patch the current inference engine into compliance.
We are replacing draft inference architecture with a systematic model that matches TypeScript semantics within our declare to export to infer framing.
Incremental fixes are acceptable only when they directly advance a refactor track or unblock the refactor itself.

## Landing Strategy: Thin Slices With Gates

This refactor must land as a sequence of reviewable slices rather than a single massive diff.
Each slice should be either scaffolding with no behavior changes, or a single migration axis that moves a well-defined subsystem onto the new contracts.
Known failures are the safety valve, but green should remain the default signal at every step.
After each slice, we should run both compiler and spec gates before proceeding.

The required gates after every slice are:

- `cargo test --release -p destack_compiler`
- `cargo test --release -p destack_test --test specification`

Nightly toolchains (currently `nightly-2025-11-27`) may emit internal compiler error logs during refactors.
- Remove `rustc-ice-*.txt` logs and do not treat them as Destack regressions.

When a slice intentionally changes behavior, update `known-failures.txt` to reflect the new baseline before continuing.
Do not let regressions accumulate across slices without being tracked explicitly in known failures.

## Execution Strategy: Foundational Order

This section defines the immediate sequencing constraint we should honor before large structural refactors.

Widening, freshness, and const contexts are cross-cutting and currently inconsistent.
They also affect utility types, mapped types, overloads, and export surfaces in non-obvious ways.
We should make widening behavior TypeScript-correct early, then refactor the surrounding architecture around that clarified contract.
Once widening is pinned down, the next steps should follow the dependency order below so correctness fixes land on stable contracts.

The foundational order for correctness work is:

- Track 3, widening and freshness first.
- Track 2, apparent types and key queries next.
- Track 1, relation engine and typing modes once apparent types are stable.
- Track 4, constraint shapes versus validation after relation modes are explicit.
- Track 5, materialization and walking to converge rewriting semantics.
- flow join and match narrowing, once relation and widening are stable.
- async and try typing, once flow joins are stable.
- static value parameter inference across modules, after constraints and materialization are stable.
- smaller correctness buckets, including struct embedding, class member availability, and default parameter inference.

Before starting the larger refactor steps below:

- Land widening rules in DESIGN and SPECIFICATION at a high level.
- Add spec coverage for widening and freshness, including cross-module and const assertion cases.
- When utility types or builtins regress, run the ignored builtin lib resolve tests in `language/compiler/src/resolve/language.rs`.
- Treat the current staged diff as a reference implementation and safety net, not as the target architecture.

## Current Test Snapshot

This section captures the current compliance signal so we do not lose track while refactoring.

On January 28, 2026, `cargo test --release -p destack_test --test specification` reports:

- 1282 passed.
- 0 regressions.
- 26 known failures.
- 26 skipped.

All current regressions are tracked as known failures to keep the unified suite stable while we refactor.
Known failures cluster heavily in overload resolution, widening, and mapped type behavior.

On January 28, 2026:

- `cargo test --release -p destack_compiler` is green.
- `cargo test --release -p destack_test --test specification` reports 1282 passed, 26 skipped, 26 known failures.

We can cross-check upstream behavior in the local TypeScript checkout at `~/symbol/TypeScript`.

## Rolling TODOs

This list should stay short and reflect the active slice we are landing.

- [x] Push literal context helpers into remaining inference paths, including argument inference, function bodies, and binding commits.
- [x] Expand widening spec coverage for function returns, conditional joins with const assertions, and nested literal contexts.
- [x] Refresh the test snapshot after the widening and context changes.
- [x] Add an explicit flow join helper for if, match, and try results, and document the contract.
- [x] Extend widening specs for contextual object literals in generics and return annotations with unions.
- [x] Consolidate return type commitment into a shared helper and apply it to expression-body lambdas.

## What A TSC Engineer Would Call Out

This section names concrete footguns and non-systematic patterns that will keep biting us unless we fix them in this refactor.

### Phase Boundary Violations

Some helpers reach across the declare, export, and infer contracts in ways that are hard to reason about.
These need to be replaced with explicit, phase-owned APIs.

- `common/mapped.rs:declared_type_for_symbol(...)` pulls declared types from remote modules to answer shape questions, which is the wrong contract for mapped types and `keyof`.
- `common/normalize.rs:apparent_type_for_assignability(...)` substitutes static parameter constraints into apparent types, which is not a TypeScript rule and changes observable keys.
- `infer/parameter.rs:static_parameter_constraint_type(...)` evaluates and imports remote constraints ad hoc and caches them locally without any explicit remote stamp or dependency tracking.
- `infer/solve.rs:materialize_infer_type_for_check(...)` performs a large custom traversal inside the solver rather than using a shared walker and explicit typing modes.
- multiple call sites still mutate declare-owned slots during infer and then call `invalidate_normalization_cache()` as a global safety valve.

### Cache Unsoundness And Global Invalidation

The current cache invalidation story is too coarse and too easy to get wrong.

- `types.invalidate_normalization_cache()` is used after many in-place mutations, which is both a performance problem and a smell that we do not have a sound invalidation model.
- `get_type_mut(...)` is called directly in many places, making it difficult to enforce mutation policy and version tracking.
- apparent types, widening, and materialization are context-sensitive, but most caches do not encode the relevant context and modes in their keys.

### Type Traversal Duplication

We are re-implementing large recursive traversals in multiple places with slightly different semantics.
This creates drift and subtle bugs.

- `common/conditional.rs` contains parallel implementations of `type_contains_static_parameters`, `type_contains_infer`, and `type_contains_infer_vars`.
- static argument materialization and infer var materialization both implement deep recursive rewrites in different modules.
- mapped type normalization contains bespoke recursion and ad hoc checks that should be declarative mode flags on a shared walker.
### Stopgaps To Remove

- Reify now skips binary implicit casts when a node has no resolution; this is correct for unresolved operator overloading but should eventually be replaced by a principled “resolved-only reify” phase gate rather than local checks.

### Missing Or Implicit Typing Modes

TypeScript’s checker is mode-heavy, and we currently hide those modes behind incidental details.

- there is no explicit representation of fresh literal types, widened literal types, or regularized literal types.
- const contexts and const assertions are not tracked explicitly in the typing context.
- apparent type queries, contextual typing, widening, and assignability all have subtly different rules, but we do not encode those rules as explicit relation or typing modes.

## Refactor Tracks That Must Land Together

This section outlines the minimum set of systemic tracks that need to move together to reach modern strict TypeScript parity.

Internal todo: track 3 widening commitments, track 2 apparent type shape entrypoints, and track 1 relation mode caching and gates.

### Track 1: Relation Engine And Typing Modes

We need a central relation engine that encodes mode and intent explicitly.
This is the spine that everything else should hang off of.

Introduce explicit relation kinds and flags, for example:

- relation kind: assignable, comparable, subtype, identical, and constraint.
- relation flags: use apparent type, skip constraint substitution, allow fresh literals, prefer non widening, and use contextual type.
- relation context: module, profile, typing mode, substitution fingerprint, and remote stamps.

All relation checks should go through this engine.
Cache keys must include relation kind, flags, and the relation context fingerprint.

Implementation notes so far:
- RelationMode now carries RelationKind + RelationFlags in `language/compiler/src/analyze/common/relation.rs`.
- Normalize now reads from `relation_mode.flags.*`.
- Added `normalize_type_with_relation(...)` and threaded relation modes through normalization and mapped type/keyof/index helpers.
- Normalization caches are now keyed by relation cache keys, not just assign or flow mode.

Internal todo:
- key normalization caches by relation flags and mode rather than a single assign or flow slot.
- unify relation flags in a central context so assignability, apparent, and flow checks do not diverge silently.
- thread relation mode into all normalize and relation entrypoints, removing implicit defaults.

Concrete contracts:
- Overload resolution uses first applicable signature in declaration order for TS++ overload sets.
- Applicability is checked with relation mode that respects contextual typing, apparent types, and constraint substitution policy.
- Union dispatch must preserve overload declaration order within each candidate and must not reorder based on resolution paths.

### Track 2: Apparent Types And Key Queries

Mapped types, `keyof`, and utility types will remain spotty until apparent type behavior is correct and centralized.

- delete declared type fallbacks for shape and key queries.
- introduce `require_instance_type(...)` and `apparent_instance_type(...)` as the only sanctioned shape entry points.
- remove constraint substitution from apparent type computation.
- model primitive apparent types explicitly via the builtin libs and apparent type helpers.

Internal todo:
- remove declared type fallbacks from mapped type and key queries and route through apparent instance helpers only.
- ensure apparent type queries never substitute constraint bounds unless explicitly requested by relation flags.
- audit `apparent_type_for_assignability` and eliminate flow-specific shape changes.

Concrete contracts:
- `keyof`, mapped type modifiers, and key remapping only operate on apparent instance types.
- Apparent type computation is pure and side effect free, never mutating declare owned data.

### Track 3: Widening, Freshness, Const Contexts, And Best Common Type

Widening must become a first class contract rather than an emergent property.
This track should land early but as part of the refactor.

- represent fresh literal types explicitly and regularize them at commitment points.
- make const context a first class part of the typing context.
- implement TSC style widened literal types and widened types.
- implement best common type and contextual typing hooks that honor freshness and const context.
- binding mutability should be recorded on symbols at bind time and never recovered by walking AST parents.
- ensure export surfaces apply the same commitment rules rather than inventing new ones.

Implementation notes so far:
- Added `LiteralFreshness`, `WideningMode`, and `ConstContext` in `language/compiler/src/analyze/common/context.rs`.
- Threaded those into `InferContext` with helpers in `language/compiler/src/analyze/infer/context.rs`.
- Began wiring const contexts and literal widening through `infer/expression.rs`.
- Added const context helpers for binding defaults, commit points, and function body entry, and applied them to argument inference and function bodies.
- Added nested expression contexts and return type commitment for inferred signatures, and materialized infer vars for member access.
- Added a flow join helper for if/match/try results to centralize best common type selection.
- Added widening specs for explicit generic unions on methods and nested return annotations.
- Added best-common-type helpers for if/match expression results with contextual expected-type gating.
- Try expressions now use best common type rather than raw unions.
- Added explicit binding commit to widen mutable bindings while preserving fresh literals for inference.
- Binding commits now widen scalar literal unions and reuse committed types for pattern bindings.
- Scalar literal contextual typing now matches numeric and int primitives via assignability checks.
- Contextual object typing now normalizes mapped shapes with TYPE_OPS relation mode.
- Numeric best common type widening now respects literal freshness and const contexts.

Internal todo:
- wire best common type through try, conditional expressions, and flow join points.
- add explicit commitment points for fresh literal regularization and const context consumption.
- ensure export surfaces and cross-module shapes apply the same widening rules.

Concrete contracts:
- Explicit commitment points: binding commits, export surface commits, flow join points (if, match, try, ternary), and contextual typing entrypoints.
- Fresh literal preservation depends on const context or const assertions, otherwise regularize and widen at commitment points.
- Best common type is the only mechanism for flow joins and match/try result typing.

### Track 4: Constraint Shapes Versus Validation

We need to separate symbolic constraint shapes from call site validation types to avoid erasing structure.

- constraint shape is declare owned and may reference static parameters symbolically.
- constraint for validation is infer owned and is only produced at explicit validation sites.
- constraint validation must not mutate constraint shapes in place.
- remote constraint resolution must be gated and stamped.

Concrete contracts:
- Constraint shapes are declare owned and must survive through export without evaluation.
- Validation types are infer owned and computed only at call/instantiation sites.
- Recursive conditional depth errors are reported during validation, not during declare shape construction.
- Static value parameter inference preserves literal values across module boundaries until validation time.

### Track 5: Materialization And Type Walking

We should not land more materialization changes without a shared walker and explicit materialization modes.

- add a unified type walker with inspection and rewrite modes.
- define materialization modes: shape, validation, and surface.
- port `type_contains_infer`, `type_contains_infer_vars`, and `type_contains_static_parameters` to the walker with explicit flags.
- port infer var materialization and static argument materialization to the same walker.
- make all materialization caches mode and context aware.

Concrete contracts:
- All deep type rewrites (infer substitution, static argument substitution, readonly materialization) go through the shared walker.
- Walker cache keys must encode mode, relation flags, and materialization mode.

### Flow Join And Async Contracts

- Flow joins (if, match, try, ternary) use best common type and apply freshness commitments at the join point.
- Await unwrapping distributes over unions and recursively unwraps nested promises before assignability checks.
- Try error typing unions error sources using best common type and preserves explicit error annotations.

Implementation notes so far:
- Added rewrite walker `language/dir/src/type/rewrite.rs` alongside the visitor walk.
- `materialize_infer_type_for_check` now uses `TypeRewriter` in `language/compiler/src/analyze/infer/solve.rs`.
- `materialize_static_arguments_in_type` now uses `TypeRewriter` in `language/compiler/src/analyze/infer/argument.rs`.
- Introduced `MaterializationMode` in `language/compiler/src/analyze/common/materialize.rs` and threaded it into both materializers.
- Moved static parameter and infer containment visitors into `language/compiler/src/analyze/common/type.rs`.
- Consolidated infer containment visitors into a single mode-driven visitor to reduce duplication.
- Conditional infer substitution now uses a `TypeRewriter` instead of bespoke recursive static argument rewrites.
- Readonly materialization now uses a `TypeRewriter` in `language/compiler/src/analyze/infer/type.rs`.

### Track 6: Mutation Policy And Cache Soundness

We need mechanical safeguards that make it hard to write cache unsound code.

- split declared versus inferred signature slots and avoid mutating declared slots during infer.
- introduce per type versions plus dependency fingerprints and remote stamps.
- add mutation helpers that bump versions and clear related caches narrowly.
- forbid direct `get_type_mut(...)` in Analyze outside mutation helpers.

Concrete contracts:
- Declare owned slots are only mutated during declare and export phases.
- Infer owned slots are only mutated through explicit mutation helpers that bump per-type versions.
- Cache invalidation is keyed by per-type version or dependency fingerprint, never global epochs.

### Track 8: Known Failure Buckets As Acceptance Gates

We should treat the current known failure clusters as acceptance gates for the refactor.

The current known failures cover:

- widening and best common type commitment points.
- apparent type and key remapping behavior.
- overload dispatch and declaration order, including unions and imported extensions.
- conditional infer edge cases with any and never, plus recursion depth reporting.
- match narrowing and match result typing.
- await unwrapping and union distribution.
- try error typing behavior.
- static argument value parameter inference across modules.
- remaining smaller buckets: discriminant narrowing through index access, struct embedding, class member availability, and function default parameter inference.

Map the known failure files to refactor tracks as acceptance gates:

- Track 3, widening and freshness.
  - `types/widening/common.md`
  - `types/widening/literals.md`
  - `flow/match/widening.md`
- Track 2, apparent types and key queries.
  - `types/generics/modifiers.md`
- Track 1, relation modes and contextual typing.
  - `resolution/dynamic/calls.md`
  - `resolution/dynamic/modules.md`
  - `types/operators/satisfies.md`
- Track 4, constraint shapes versus validation.
  - `types/generics/infer.md`
  - `types/operators/conditional.md`
  - `types/generics/recursion.md`
- Track 1 to 3, flow join and match narrowing.
  - `flow/match/narrowing.md`
  - `flow/narrowing/discriminant.md`
- Track 1 to 4, async and try typing.
  - `expressions/async/promise.md`
  - `expressions/try/catch.md`
- Track 4 to 5, static value parameter inference across modules.
  - `types/static-arguments/value-parameters.md`
- Track 2 to 6, remaining buckets.
  - `types/structs/embedding.md`
  - `declarations/classes/members.md`
  - `declarations/functions/inference.md`

The widening regressions in `types/widening/literals.md` are also acceptance gates and should be resolved by Track 3 rather than by patching tests.

### Track 9: Remote Access Normalization

Cross-module access should be centralized rather than reimplemented ad hoc across Analyze.
We should introduce a small remote access layer that owns require gating, type evaluation, imports, and error handling.

- add a shared helper that resolves remote module access and returns a local view or imported type id.
- stop calling `module.dir(profile)` and `remote_dir.types.write()` directly outside the helper.
- stamp and cache remote imports consistently so callers do not need to re-run evaluation logic.
- make it explicit which phase owns a remote read and what invariants it can assume.
- audit every `let remote_module = ...` site in Analyze and fold it into the helper.
- address the existing FUGU note about local versus remote splits in `common/conditional.rs`.

## Acceptance Checklist

This section maps each remaining failure cluster to the refactor tracks that should resolve it.
The order below mirrors the foundational order so we burn down the largest buckets with the least thrash.

- widening, freshness, and best common type commitment points: Track 3.
  - `language/test/fixtures/specification/types/widening/literals.md`
  - `language/test/fixtures/specification/types/widening/common.md`
  - `language/test/fixtures/specification/flow/match/widening.md`
- apparent types and key queries: Track 2.
  - `language/test/fixtures/specification/types/operators/apparent.md`
  - `language/test/fixtures/specification/types/generics/modifiers.md`
- relation modes and phase contracts: Track 1.
  - `language/test/fixtures/specification/resolution/dynamic/calls.md`
  - `language/test/fixtures/specification/resolution/dynamic/modules.md`
  - `language/test/fixtures/specification/types/operators/satisfies.md`
- constraint shapes versus validation: Track 4.
  - `language/test/fixtures/specification/types/generics/infer.md`
  - `language/test/fixtures/specification/types/operators/conditional.md`
  - `language/test/fixtures/specification/types/generics/recursion.md`
- walker based materialization and deep rewrites: Track 5.
  - `language/test/fixtures/specification/expressions/async/promise.md`
  - `language/test/fixtures/specification/expressions/try/catch.md`
  - `language/test/fixtures/specification/types/static-arguments/value-parameters.md`
- flow join and match narrowing: Track 1 to 3.
  - `language/test/fixtures/specification/flow/match/narrowing.md`
  - `language/test/fixtures/specification/flow/narrowing/discriminant.md`
- remaining smaller buckets: Track 2 to 6.
  - `language/test/fixtures/specification/types/structs/embedding.md`
  - `language/test/fixtures/specification/declarations/classes/members.md`
  - `language/test/fixtures/specification/declarations/functions/inference.md`
- mutation policy and cache soundness: Track 6.
  - protects all later buckets from cache churn and invalidation storms.
- overload dispatch and declaration order: Track 6, after Tracks 1 to 5 land.
  - `language/test/fixtures/specification/resolution/overloads/functions.md`
  - `language/test/fixtures/specification/resolution/overloads/modules.md`
  - `language/test/fixtures/specification/resolution/overloads/namespace.md`
  - `language/test/fixtures/specification/resolution/overloads/generics.md`
  - `language/test/fixtures/specification/resolution/overloads/context.md`
  - `language/test/fixtures/specification/resolution/overloads/methods.md`
  - `language/test/fixtures/specification/resolution/overloads/constructors.md`
  - `language/test/fixtures/specification/resolution/overloads/call.md`
  - `language/test/fixtures/specification/resolution/overloads/ambiguity.md`
  - `language/test/fixtures/specification/resolution/overloads/this.md`
  - `language/test/fixtures/specification/resolution/overloads/static.md`
  - `language/test/fixtures/specification/resolution/overloads/receiver.md`
  - `language/test/fixtures/specification/resolution/overloads/overlap.md`
  - `language/test/fixtures/specification/resolution/dynamic/calls.md`
  - `language/test/fixtures/specification/resolution/dynamic/modules.md`
- discriminant narrowing through index access: Tracks 1 to 3.
  - `language/test/fixtures/specification/flow/narrowing/discriminant.md`
  - `language/test/fixtures/specification/flow/match/narrowing.md`
- conditional infer for `any` and `never`: Tracks 1, 4, and 5.
  - `language/test/fixtures/specification/types/generics/infer.md`
- conditional types with `any`: Tracks 1 and 4.
  - `language/test/fixtures/specification/types/generics/infer.md`
  - `language/test/fixtures/specification/types/operators/conditional.md`
- struct embedding through nested embeds: Tracks 2 and 6.
  - `language/test/fixtures/specification/types/structs/embedding.md`
- builtin lib regressions and utility types: Tracks 2, 4, and 5.
  - `language/test/fixtures/specification/types/operators/apparent.md`
  - `language/test/fixtures/specification/types/generics/utility-types.md`
  - `language/test/fixtures/specification/types/generics/modifiers.md`
  - `language/test/fixtures/specification/types/generics/recursion.md`
  - `language/test/fixtures/specification/types/static-arguments/value-parameters.md`

Each checklist line should be backed by spec coverage and, where necessary, targeted invariant unit tests.

## Current Divergences From TSC

This section records observed mismatches that the widening refactor must correct.

- Apparent type resolution is currently too weak and too opinionated.
- `apparent_type(...)` only unwraps cached alias normalization instances.
- `apparent_type_for_assignability(...)` substitutes static parameter constraints into apparent types.
- Substituting constraints into apparent types is not a TSC rule and can erase useful structure.
- Widening is currently ad-hoc and mostly numeric.
- Literal inference currently yields literal types, but there is no explicit freshness model and no explicit widened literal model.
- Widening and apparent type resolution are both context-sensitive, but current caches and helpers do not consistently encode that context.

## Terminology Alignment With TSC

This section aligns naming and concepts with the TypeScript compiler where it helps clarity and correctness.

We should prefer TSC terminology in APIs and docs when we implement TSC semantics.
This reduces ambiguity and makes it easier to compare behavior to upstream.

Key terms to adopt include:

- apparent type
- contextual type and contextual typing
- fresh literal type and widened literal type
- const assertion and const context
- assignability and relation
- instantiation and inference candidates
- overload resolution and signature

Where Destack and TS++ diverge, we should document the divergence next to the TSC term we are borrowing.

For upstream semantics and naming, we can cross-check against the local TypeScript checkout at `~/symbol/TypeScript`.

## Phase Contracts

This section formalizes the contracts that all helper methods must respect.

### Declare Contract

Declare must fully establish all shape-level facts that other phases can safely depend on.

- All declarations have a declared type slot, even if it is `unknown` or symbolic.
- All instantiable symbols have an instance type slot populated with a stable shape.
- All function and method declarations have a signature type slot populated with a stable signature shape.
- All static parameters have:
  - a kind slot populated, and
  - a constraint shape slot populated symbolically.
- Declare may evaluate type expressions, but it must not perform bound validation that depends on call-site substitutions.
- Declare must not depend on infer results, and must be able to run to a local fixpoint using only declare and resolve data.

### Export Contract

Export must compute surfaces without forcing inference or performing remote peeking.

- Export may depend on declare results from other modules via require gates.
- Export must never trigger inference in a different module.
- Export must only publish declared and surface-inferred shapes that are stable under the declare contract.

### Infer Contract

Infer is the only phase allowed to compute value-level types and solve inference variables.

- Infer may depend on declare and export results from other modules via require gates.
- Infer must not mutate declared shapes, lineages, or other declare outputs in place.
- Infer may compute refined signature and value types, but refinements must be written to infer-owned slots.
- Infer must be the only phase that validates static argument bounds at call sites and instantiation sites.
- Infer owns contextual typing, widening, narrowing, and overload resolution.

## Cross-Module Access Contract

This section defines how any code may read data from other modules.

All cross-module reads must follow a three-step pattern.

1) Require the phase that owns the data.
2) Read the remote tables.
3) Import the remote type into the local type table.

The require step must be explicit and reflect the phase contract.

- For shapes: require declare.
- For export surfaces: require export.
- For inference results: require infer, but this should be rare and carefully justified.

No helper should directly read a remote table without first calling an explicit `require_*` method.

## Static Parameter Constraints

This section defines the final constraint model and the APIs that implement it.

### Constraint Model

Constraints have two distinct forms with different responsibilities.

- Constraint shape: a symbolic type expression that may reference static parameters.
- Constraint for validation: a substituted and normalized type that is ready for assignability checks.

Constraint shape belongs to declare.
Constraint validation belongs to infer.

### Constraint APIs

This subsection defines the target APIs and where they should live.

Create a dedicated constraint module, such as `language/compiler/src/analyze/common/constraint.rs`.

Expose two primary entry points.

- `static_parameter_constraint_shape(...) -> LocalTypeId`
  - Requires declare for the owning module.
  - Ensures the constraint is evaluated into a symbolic shape.
  - Never performs bound validation.
  - May import the remote constraint shape into the local table.
- `static_parameter_constraint_for_validation(...) -> LocalTypeId`
  - Requires the constraint shape.
  - Applies explicit substitutions.
  - Materializes nested constraint shapes as needed.
  - Normalizes the result for assignability.
  - Performs no caching that would be invalid under different substitutions.

All existing call sites should be migrated to one of these two forms explicitly.

## Instance Types As The Only Shape Source

This section removes declared-type fallbacks as a concept.

The only sanctioned source of object keys and structural shapes is the apparent instance type.

Make this invariant explicit.

- If we need shape information for a symbol, we must obtain an instance type id.
- If the instance type is not present locally, we must require declare and then resolve or import the instance type.
- We must not fall back to declared types to approximate instance shapes.

### Apparent Types

TypeScript uses apparent types for key queries and mapped types.
Apparent types are not always the same as the raw instance slot.
Type aliases often want their alias target as the apparent type.
Primitive wrappers and special library shapes often want a library apparent type.
Mapped key extraction should operate on the apparent type, not on a raw reference.

Define an explicit apparent type helper, and make it the only path for key extraction.

- `apparent_instance_type(...) -> Option<LocalTypeId>`
  - Requires declare for remote modules.
  - Resolves or imports the instance type.
  - Expands alias targets when needed for key queries.
  - Applies a shallow apparent-type normalization that preserves static parameters.
  - Never performs validation that depends on call sites.

Apparent types for primitives and well-known library shapes should be modeled explicitly.
This should not be left as a procedural special-case scattered across helpers.
Apparent type helpers should align with TSC naming and usage guidance.
When we need members, we should use the equivalent of TSC `getApparentTypeOfContextualType`.
When we are just propagating a contextual type, we should use the equivalent of TSC `getContextualType`.

### Required API

Define a single helper that enforces the invariant.

- `require_instance_type(...) -> Option<LocalTypeId>`
  - Requires declare for remote modules.
  - Resolves instance types via existing merge-group logic.
  - Imports remote instance types into the local table.
  - Never reads remote tables without a require gate.

All shape consumers should call the apparent type helper, which may call this helper internally.

Candidate consumers include:

- `common/mapped.rs` key extraction for `keyof` and mapped types.
- `common/normalize.rs` alias resolution fallback paths.
- `infer/type.rs` and `infer/assign.rs` shape-based checks.

## Widening, Freshness, And Const Contexts

This section captures TypeScript widening and freshness rules using TSC terminology.

Widening and freshness are a major source of subtle incompatibilities.
These rules must be explicit and phase-owned rather than emergent from normalization.

### Widening Contract

Widening decisions should happen at explicit commitment points, not during generic type evaluation.

- Fresh literal types should be created for literal expressions and literal-like object and tuple expressions.
- Widening should occur when a fresh literal type escapes a const context without a constraining contextual type.
- Widening should not occur inside constraint shapes, mapped key extraction, or other type-level shape construction.
- Export surfaces should preserve fresh literal information only when doing so matches TSC behavior, and otherwise publish widened stable surfaces.
- Commitment points should be explicit and enumerable, for example:
  - variable declarations without constraining annotations or contextual types
  - array and tuple best-common-type construction in non-const contexts
  - return statements for functions without explicit return types and without constraining contextual signatures
  - export surface publication
  - inference finalization when candidates must become stable surface types

Widening rules should be specified explicitly in design and specification docs, and validated with spec tests, including cross-module cases.

### Const Contexts And Const Assertions

Const contexts should be explicit in the typing context.

- Const assertions should produce non-widening literal types and readonly object and tuple shapes.
- Const contexts should propagate through contextual typing in places where TSC treats expressions as const contexts.
- Constness must be tracked in the typing context rather than inferred from incidental details like binding kind alone.

### Suggested API Surface

Widening and freshness should be centralized to prevent drift.

- `is_const_context(...) -> bool`
- `get_contextual_type(...) -> Option<LocalTypeId>`
- `get_apparent_type(...) -> LocalTypeId`
- `get_apparent_type_of_contextual_type(...) -> Option<LocalTypeId>`
- `fresh_literal_type(...) -> LocalTypeId`
- `is_fresh_literal_type(...) -> bool`
- `get_regular_type_of_literal_type(...) -> LocalTypeId`
- `get_base_type_of_literal_type(...) -> LocalTypeId`
- `get_widened_literal_type(...) -> LocalTypeId`
- `get_widened_type(...) -> LocalTypeId`

These names mirror TSC terminology and should live in a shared type system utility module.
The implementation should follow TSC structure closely enough to make behavior comparisons easy in the local upstream checkout.

### Freshness Representation

Freshness should be representable without leaking fresh-only types into long-lived surfaces.

We should explicitly represent fresh versus regular literal types.
This can be done with wrapper nodes, flags, or side tables, but the representation must be visible to cache keys and rewriting.

Freshness must be stripped or regularized at commitment points before caching and export surface publication.
Freshness must not be silently erased inside normalization helpers.

## Unified Type Walker

This section eliminates duplication across many `type_contains_*` and materialization functions.

We already have a shared walker and rewriter in `language/dir/src/type/walk.rs` and
`language/dir/src/type/rewrite.rs`.
The remaining work is to extend them with explicit modes and cache keys, then route
Analyze utilities through a small analyze-layer wrapper so we stop re-implementing
ad-hoc visitors and rewriters.

### Walker Capabilities

The walker should support both inspection and rewriting.
We already have both, but they are missing mode-aware context and cache keys.

- A cached structural walk over `Type` graphs.
- Hooks for:
  - pre-visit decisions,
  - leaf handling, and
  - post-visit reconstruction.
- A rewrite mode that reuses existing nodes when nothing changes.
- A context object that makes mode and substitution state explicit.
- Cache keys that include the walker mode, substitution fingerprint, and relevant options.
- Cache keys that include const context and widening mode when those affect the result.

### Walker Utilities

This subsection lists the utilities that should be built on the walker.

Predicates:

- `type_contains_static_parameters(...)`
- `type_contains_infer_patterns(...)`
- `type_contains_infer_vars(...)`
- `type_contains_kind(...)` as a generic utility

Rewriters:

- `rewrite_materialize_static_arguments(...)`
- `rewrite_materialize_infer_vars_for_check(...)`
- `rewrite_substitute_static_parameters(...)` can be ported to the walker model if useful

### De-duplication Targets

This subsection names concrete duplication to eliminate.

- `common/conditional.rs`: `type_contains_static_parameters`, `type_contains_infer`, and `type_contains_infer_vars`
- `infer/argument.rs`: `materialize_static_arguments_in_type` and nested materialization helpers
- `infer/solve.rs`: `materialize_infer_type_for_check(_inner)`
- `common/mapped.rs`: repeated ad-hoc contains checks

The goal is one shared traversal with thin wrappers.

Current status:

- `language/dir/src/type/walk.rs` provides `TypeVisitor` based traversal.
- `language/dir/src/type/rewrite.rs` provides `TypeRewriter` based rewrites.
- `TypeVisitorOptions` and `TypeRewriterOptions` now carry cache keys.
- `TypeWalkContext` exists in `language/compiler/src/analyze/common/walk.rs` and is used by the infer and static argument materializers to set rewrite options.
- Base visitor options now flow through `TypeWalkContext` for common walker utilities.
- Containment walkers are unified behind `TypeContainmentVisitor` in `language/compiler/src/analyze/common/type.rs`.
- Materialization rewriters share `rewrite_type_with_cache(...)` with `TypeRewriteCache` keyed by walk modes.
- Analyze still has multiple bespoke visitors and rewriters in:
  - `language/compiler/src/analyze/common/type.rs`
  - `language/compiler/src/analyze/infer/argument.rs`
  - `language/compiler/src/analyze/infer/solve.rs`
  - `language/compiler/src/analyze/common/conditional.rs`

Next work:

- add mode and cache key support to the existing walker and rewriter rather than adding a new module.
- add a small analyze-layer wrapper to standardize walker usage and cache keys.
- migrate the contains checks and materializers to the shared walker wrapper.

## Normalization Cache Redesign

This section addresses the current global-epoch invalidation pattern.

The current cache is too coarse because we mutate types in place.
The final form should make invalidation local and explicit.

### Desired Invariants

This subsection states the invariants that make caching safe.

- Normalization results depend on the structural identity of referenced types.
- When a type changes, we must invalidate normalization results that depend on that specific type.
- We must not invalidate unrelated normalization entries.

### Proposed Design

This subsection defines a concrete, implementable design.

Replace the global normalization epoch with per-type versions.

- Add `type_version_by_id: Vec<u64>` to `TypeTable`.
- Increment the version whenever a type is mutated in place.
- Store the observed version alongside cached normalization entries.

Add dependency tracking so cached results are invalidated when any dependency changes.

- Track a compact dependency fingerprint during normalization.
- The fingerprint can be a hash of `(type_id, type_version)` pairs visited during normalization.
- Store the fingerprint alongside the cached normalized type id.
- Recompute the fingerprint on cache lookup, and reuse only when the fingerprint matches.
- Avoid storing large dependency sets by using a rolling hash that is stable across traversal order.

Update cache shapes.

- For `normalized_assignability_type_by_id`, also store `normalized_assignability_version_by_id`.
- For `normalized_flow_type_by_id`, also store `normalized_flow_version_by_id`.
- For alias normalization caches, store a compact dependency fingerprint or observed versions for the root type and alias target.

Introduce explicit mutation helpers.

- `replace_type_in_place(ty_id, new_ty)` should:
  - update the type,
  - bump that type version, and
  - clear alias normalization entries for related symbols when necessary.

Then, forbid direct `get_type_mut` in Analyze code except inside these helpers.

### Cross Module Invalidation

Imported types must be invalidated when their remote sources change.
Remote invalidation is a common compiler footgun in parallel pipelines.
TypeTable should store a remote profile or module stamp for imported types.
Require gates should clear imported types whose source stamps are stale before new imports are inserted.
Normalization should include remote stamps in its dependency fingerprint.

Widening should not be hidden inside normalization, because widening is context-sensitive.
Normalization should consume already widened or already fresh types based on the caller's explicit mode.

### Mutation Policy

This subsection defines when in-place mutation is allowed.

In-place mutation is allowed only for declare-owned slots during declare, or infer-owned slots during infer.

Declare-owned slots:

- declared types
- instance types
- signature types
- constraint shapes

Infer-owned slots:

- inferred types
- resolutions
- instances

Cross-phase mutation of declare-owned slots is forbidden.

This rule prevents silent cache unsoundness and phase-ordering bugs.

## Static Argument Materialization Refactor

This section unifies the many `materialize_static_*` variants.

### Materialization Modes

Define a single materialization pipeline parameterized by an explicit mode.

- `MaterializationMode::Shape`
  - Materializes unevaluated arguments into typed shapes using parameter kinds.
  - Uses constraint shapes only.
  - Never performs bound validation.
- `MaterializationMode::Validation`
  - Uses constraint-for-validation.
  - Substitutes nested constraints as needed.
  - Normalizes for assignability.
  - Emits diagnostics for invalid arguments.

Add an explicit surface mode for export and shape construction.

- `MaterializationMode::Surface`
  - Produces stable export surfaces.
  - Preserves symbolic constraints.
  - Avoids call-site validation.
  - Avoids leaking inference variables across module boundaries.

### Materialization API

Define a single entry point and a single type rewriter.

- `materialize_static_arguments_for_reference(mode, ...) -> Vec<StaticArgument>`
- `materialize_static_arguments_in_type(mode, ...) -> LocalTypeId`

These should be thin wrappers over the unified type walker.

The materialization cache must be keyed by the mode and a substitution fingerprint.
Reusing a cache across modes is a correctness footgun.

### Ownership Of Argument Nodes

Static arguments now carry global node ids.
Argument ownership should be resolved via a single helper with a clear contract.

- Keep `with_static_argument_owner(...)`, but move it to a shared utility module.
- Require the owning module declare phase before reading its tree when needed.
- Ensure all error nodes are anchored to the call-site node, not the remote argument node.
- Ensure the walker carries a call-site anchor so diagnostics are stable even when nodes are remote.

## Function Value Type Merging

This section clarifies overload merging without changing semantics.

The existing `merge_function_value_type(...)` approach is directionally correct for TypeScript-style overloads.
The main improvement is to make the contract explicit and guard it.

### Contract

State the intended behavior in documentation and assertions.

- Multiple declarations for the same symbol produce a value type with multiple call signatures.
- When re-inferring the same declaration, the previous signature from that declaration is replaced.
- Duplicate overloads are diagnosed in non-declaration modules.

### Improvements

Make the existing behavior more robust.

- Add debug assertions that the signature we are replacing was previously recorded for that declaration.
- Avoid cross-phase in-place mutation of the same signature type id.
- Split declared signature types from inferred signature types and avoid mutating declared signature slots during infer.

### Signature Slot Split

Declared and inferred signatures should live in different slots.
This makes the phase contract mechanically enforceable.

Suggested slots include:

- declared signature type
- inferred signature type
- apparent signature type for export surfaces when needed

## Test Strategy

This section defines where coverage should live after the refactor.

### Spec Tests First

Prefer spec tests for observable behavior.

Add or extend spec coverage for:

- utility types with invalid keys
- keyof in declare-only contexts
- cross-module generic constraints
- overload selection and declaration order
- conditional types with `any`, `never`, and distributivity edge cases
- mapped types that depend on symbolic constraints

### Unit Tests For Invariants

Use unit tests only for internal invariants that the spec cannot express cleanly.

Examples include:

- per-type normalization cache invalidation behavior
- type walker caching behavior on recursive types
- constraint shape versus constraint-for-validation separation
- apparent type behavior for key extraction on aliases and primitives
- cache soundness under in-place mutation and remote invalidation
- widening and const context behavior for fresh literals, const assertions, and contextual typing

## Execution Roadmap

This section is the authoritative step-by-step order for the refactor.
Each step should land with acceptance checks before moving on.
The goal is to avoid getting lost while still allowing large structural improvements.

### Step 0: Guardrails And Safety Rails

Add mechanical safeguards before semantic changes.

- add explicit mutation helpers in `TypeTable` and start routing new mutations through them.
- add per-type versions, even if dependency fingerprints are not wired yet.
- add lightweight relation and walker context structs that can carry mode and stamps.
- add debug assertions for phase ownership, for example preventing infer from mutating declare-owned slots.

Acceptance checks:

- no behavior change.
- spec suite still passes at the current baseline except for known widening regressions.

### Step 1: Relation Engine Skeleton With Modes

Introduce a central relation entry point without changing behavior.
This is about call-site consolidation first, not semantics.

- add a relation engine module, for example `common/relation.rs`.
- define relation kinds and flags that encode intent and mode explicitly.
- make `is_type_assignable(...)` a thin wrapper over the relation engine.
- thread relation context through assignability call sites without changing the decisions yet.

Acceptance checks:

- no new regressions.
- relation caches include relation kind, flags, and a context fingerprint.

### Step 2: Unified Type Walker

Land the walker early so the rest of the refactor has a safe substrate.

- extend `language/dir/src/type/walk.rs` and `language/dir/src/type/rewrite.rs` with mode-aware options.
- add an analyze wrapper that carries walker context and cache keys.
- port `type_contains_*` utilities onto the walker first.
- port `materialize_infer_type_for_check(...)` onto the walker as a mode based rewrite.
- keep old helpers as thin wrappers to reduce churn.

Acceptance checks:

- no behavior change.
- walker caches include mode and substitution fingerprints.

### Step 3: Apparent Types And Shape Entry Points

Fix key queries and mapped types by fixing apparent types and shape contracts.

- introduce `require_instance_type(...)` and `apparent_instance_type(...)`.
- delete declared type fallbacks for key queries and mapped types.
- remove constraint substitution from apparent type computation.
- migrate mapped type key extraction and `keyof` onto apparent instance types.

Acceptance checks:

- utility types and mapped types do not regress.
- run ignored builtin lib resolve tests when utility types or builtin members look suspicious.

### Step 4: Constraint Shapes Versus Validation Types

Separate symbolic constraint shapes from call-site validation.
This prevents structure erasure and aligns with phase contracts.

- introduce a constraint module, for example `common/constraint.rs`.
- define constraint shape APIs that are declare owned.
- define constraint for validation APIs that are infer owned.
- migrate `static_parameter_constraint_type(...)` call sites to the new APIs.

Acceptance checks:

- conditional infer behavior for `any` and `never` is not worsened.
- constraint caches are mode aware and do not substitute constraints into apparent types.

### Step 5: Materialization Modes And Solver Purification

Unify materialization and make the solver mostly pure.
The solver should depend on relation modes rather than performing bespoke rewriting.

- define materialization modes: shape, validation, and surface.
- migrate static argument materialization to mode based walker rewrites.
- update the solver to produce an `InferSolution` without mutating declare-owned slots.
- restrict `apply_solution(...)` to infer-owned slots and route mutations through mutation helpers.

Acceptance checks:

- no new inference cycles across modules.
- no new global cache invalidations.

### Step 6: Widening, Freshness, Const Contexts, And Best Common Type

Implement TypeScript-correct widening using the new relation and walker infrastructure.
This step should resolve the widening regressions and many incidental incompatibilities.

- represent fresh literal types explicitly and regularize them at commitment points.
- make const context explicit in the typing context.
- implement widened literal types and widened types.
- implement best common type and contextual typing hooks that honor freshness and const context.
- ensure export surfaces apply the same commitment rules.

Acceptance checks:

- all widening regressions in `types/widening/literals.md` are green.
- no regressions in overloads, utility types, or export inference surfaces.

### Step 7: Signature Slot Split And Overload Stability

Make phase ownership mechanically enforceable for signatures.

- split declared versus inferred signature slots.
- ensure overload merging only mutates the inferred signature slot.
- add assertions that per-declaration signatures are replaced rather than stacked incorrectly.

Acceptance checks:

- overload dispatch and overload order behavior improves or stays stable.
- no new signature related cache invalidations.

### Step 8: Dependency Fingerprints And Remote Stamps

Finish the cache soundness story with dependency fingerprints and remote stamps.

- add dependency fingerprints to normalization caches.
- include remote stamps and typing modes in cache keys.
- clear stale imported types when remote stamps change.

Acceptance checks:

- spec suite stays stable under repeated runs and mixed module orderings.
- performance improves or stays within acceptable bounds.

### Step 9: Acceptance Gates And Plan Closure

Drive the remaining known failures to zero using the new architecture rather than new heuristics.

- resolve known failure buckets using relation modes and explicit contracts.
- prune or rewrite unit tests that encode old incidental behavior.
- add spec tests for any newly clarified TypeScript behavior, including cross-module cases.

Acceptance checks:

- known failures are empty.
- skipped tests are intentionally skipped and documented, or unskipped and green.

## Concrete Refactor Steps

This section lays out the recommended execution order.

The sequence is designed to keep the system working while moving to the final form.

The roadmap above is authoritative.
These concrete steps are the main structural transformations that implement the roadmap.

1) Introduce the unified type walker in a new module without changing existing call sites.
2) Port `type_contains_*` predicates onto the walker.
3) Introduce constraint shape and constraint-for-validation APIs, and migrate existing constraint call sites.
4) Implement per-type normalization cache versions and replace global invalidation calls with mutation helpers.
5) Introduce `require_instance_type(...)` and migrate shape consumers away from declared-type fallbacks.
6) Collapse static argument materialization into mode-based entry points backed by the walker.
7) Tighten infer mutation policy by avoiding in-place mutation of declare-owned slots during infer.
8) Prune redundant unit tests and move behavioral coverage into the spec suite.

Add two explicit steps for apparent types and cache dependency fingerprints.

9) Introduce `apparent_instance_type(...)` and migrate key extraction and mapped types to it.
10) Add dependency fingerprints to normalization caches and include remote stamps in those fingerprints.
11) Introduce explicit freshness and widening helpers using TSC terminology, and migrate widening decisions to explicit commit points.
12) Split declared and inferred signature slots, and migrate signature mutation to the inferred slot.
13) Split constraint shapes into their own slot rather than mutating declared type slots.

## Target Files And Modules

This section lists the primary locations that should change.

Core additions:

- `language/compiler/src/analyze/common/walk.rs`
- `language/compiler/src/analyze/common/constraint.rs`
- `language/compiler/src/analyze/common/widen.rs`

Core migrations:

- `language/compiler/src/analyze/common/conditional.rs`
- `language/compiler/src/analyze/common/mapped.rs`
- `language/compiler/src/analyze/common/normalize.rs`
- `language/compiler/src/analyze/infer/argument.rs`
- `language/compiler/src/analyze/infer/solve.rs`
- `language/compiler/src/analyze/infer/parameter.rs`
- `language/dir/src/type/table.rs`

Likely call-site touch points:

- `language/compiler/src/analyze/infer/call.rs`
- `language/compiler/src/analyze/infer/type.rs`
- `language/compiler/src/analyze/infer/assign.rs`
- `language/compiler/src/analyze/common/shape.rs`

New call-site touch points for widening and signature slot splits:

- `language/compiler/src/analyze/infer/expression.rs`
- `language/compiler/src/analyze/infer/declaration.rs`
- `language/compiler/src/analyze/infer/call.rs`
- `language/compiler/src/analyze/common/normalize.rs`

## Non-Goals

This section sets boundaries to prevent scope creep.

- This refactor does not attempt to change TypeScript compatibility semantics.
- This refactor does not attempt to remove declaration merging.
- This refactor does not attempt to eliminate all in-place mutation, only to constrain it and make it safe.

## Compatibility Footguns And Hazards

This section summarizes subtle TypeScript compatibility and performance traps that the refactor must avoid.

- Key queries must use apparent types, not raw references, especially for aliases and primitives.
- Constraint shapes must not be normalized into `any` or `unknown` prematurely, because this erases constraints and breaks utility types.
- Materialization caches must not be reused across modes, substitutions, or module contexts.
- Remote reads must always be gated by the owning phase, or parallelism will introduce racey and stale reads.
- Per-type versions without dependency fingerprints will be unsound, because normalization depends on transitive types.
- In-place mutation of declare-owned slots during infer will silently invalidate caches and violate phase contracts.
- Export surfaces must never depend on infer results, even indirectly through helpers that trigger infer in other modules.
- Shape helpers that pull remote tables without importing the result into the local table will create identity splits and brittle equality checks.
- Widening done during normalization or type-level evaluation will erase freshness and break const assertion behavior.
- Apparent types and widening are both context-sensitive, so caches must include their mode and context in their cache key.
- Apparent type resolution should not substitute constraint types as a general rule, because that changes observable structure and keys.

## Decisions

This section records decisions that are easy to regress without an explicit statement.

- Apparent types for primitives and well-known library types should be modeled explicitly, not left as procedural special-cases.
- Declared signature types and inferred signature types should be split into separate slots.
- Constraint shapes should be stored in a dedicated slot, not by mutating declared type slots in place.
- Widening should be explicit and context-owned, and not hidden inside normalization.

Open questions should be moved to spec and design documents where they can be resolved against TS behavior.
