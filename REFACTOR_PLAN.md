# Refactor Plan: Final Artifact-First Compiler Design

This plan defines the intended final compiler architecture.
It is not a transitional design.
Every slice should move directly toward this shape and delete anything that does not fit it.

## Canonical Architecture

This section is the authoritative target.
If later sections disagree with this one, this section wins.

The goal is a gold-standard incremental compiler model that feels obvious to anyone familiar with rustc, Salsa-style query systems, Swift request evaluation, or TypeScript builder architecture.

### Core model

The final architecture is a demand-driven artifact graph.

There are only two classes of semantic things:

- input facts
- derived artifacts

Inputs are versioned leaves.
Artifacts are immutable derived products.
The scheduler exists only to realize artifact requirements.
Persistence exists only to hydrate and store artifacts and input metadata.

### Final nouns

The minimum architectural nouns are:

- `Session`
- `Program`
- `FileRegistry`
- `ModuleRegistry`
- `PackageRegistry`
- `TsConfigRegistry`
- `ProfileRegistry`
- `ArtifactRegistry`
- `ArtifactStore`
- `ArtifactKey`
- `ArtifactDependency`
- `ArtifactRequirement`
- `Task`

Everything else is guilty until proven necessary.

The final architecture should not need:

- `OutputRegistry`
- `OutputKey`
- `OutputStore`
- `BuildKey`
- `BuildRequirement`

Those may still exist in the tree during transition, but they are not destination nouns.

### Ownership

The intended final owners are:

- `Session`: persistence metadata, workspace metadata, filesystem, cache I/O
- `Program`: live input registries and the authoritative published artifact state
- `Compiler`: scheduler state, transient diagnostics, and only the smallest unavoidable compile-local memoization
- phase entrypoints: plain local mutable variables only

No other shared mutable semantic state should exist.

### Input rule

Input files are not artifacts.
They are versioned leaves of the graph.

That means:

- source modules live in `FileRegistry`
- config files live in `FileRegistry`
- manifest files live in `FileRegistry`
- parsed and normalized input meaning lives in the input registries
- emitted files do not live in `FileRegistry` unless they are later re-opened as real inputs

The intended split is:

- `FileRegistry`: raw external file facts
- input registries: parsed and normalized input meaning
- `ArtifactRegistry`: everything derived

### Scheduler rule

The scheduler should know only:

- exact artifact keys
- exact dependency stamps
- queued work
- yielded waiters
- completion and failure

Phase names are telemetry vocabulary only.
They are not semantic truth and should not be the core scheduling model.

### Invalidation rule

Invalidation is version-driven and dependency-driven first.

That means:

- inputs and profiles bump versions
- expected dependency stamps change
- stale artifacts become unavailable immediately
- physical eviction is optional and secondary

Availability is determined by exact dependency validity, not eager deletion.

### Performance rule

The performance model is structural sharing plus local mutation.

That means:

- published artifacts are immutable
- unchanged fields are reused by `Arc`
- mutated fields are cloned locally only when a phase really mutates them
- tasks never mutate shared semantic truth in place
- parallelism comes from independent artifact realization, not lock-heavy shared mutable state

Minimal duplication does not mean never cloning.
It means never cloning unchanged state and never sharing mutable semantic state.

## Ordered Transition Plan

This is the ordered work needed to reach the canonical model.

### 1. Lock the input boundary

Files must remain input truth only.

Targets:

- [language/source/src/file.rs](/Users/florian/symbol/destack-4/language/source/src/file.rs)
- [language/workspace/src/program/program.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/program.rs)
- [language/workspace/src/program/module.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/module.rs)
- [language/workspace/src/program/package.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/package.rs)
- [language/workspace/src/program/tsconfig.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/tsconfig.rs)
- [language/workspace/src/program/invalidate.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/invalidate.rs)

Required end state:

- all raw source, config, and manifest files live in `FileRegistry`
- parsed and normalized input meaning lives in the input registries
- no derived semantic or emitted state is parked on files
- file version bumps are the leaf invalidation mechanism for downstream artifacts

### 2. Collapse outputs into artifacts

Derived outputs must stop being a second architectural system.

Targets:

- [language/workspace/src/artifact/registry.rs](/Users/florian/symbol/destack-4/language/workspace/src/artifact/registry.rs)
- [language/workspace/src/output/registry.rs](/Users/florian/symbol/destack-4/language/workspace/src/output/registry.rs)
- [language/workspace/src/artifact/key.rs](/Users/florian/symbol/destack-4/language/workspace/src/artifact/key.rs)
- [language/workspace/src/output/key.rs](/Users/florian/symbol/destack-4/language/workspace/src/output/key.rs)
- [language/compiler/src/generate](/Users/florian/symbol/destack-4/language/compiler/src/generate)
- [language/compiler/src/link](/Users/florian/symbol/destack-4/language/compiler/src/link)

Required end state:

- emitted families move from `OutputRegistry` into `ArtifactRegistry`
- `OutputKey` families are replaced by emitted `ArtifactKey` families
- `OutputStore` collapses into `ArtifactStore`
- generate and link publish artifacts directly
- user-visible filesystem writes are final emit side effects, not a second truth system

### 3. Collapse scheduler vocabulary to artifacts

The scheduler should run on artifacts only.

Targets:

- [language/compiler/src/compile/build.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/build.rs)
- [language/compiler/src/compile/task.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/task.rs)
- [language/compiler/src/compile/queue.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/queue.rs)
- [language/compiler/src/compile/process.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/process.rs)
- [language/compiler/src/compile/availability.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/availability.rs)
- [language/compiler/src/compile/commit.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/commit.rs)

Required end state:

- `BuildKey` is deleted
- `BuildRequirement` is replaced by `ArtifactRequirement`
- availability and commit logic run on artifacts only
- `TaskPhase` remains only if it still earns its keep as telemetry vocabulary

### 4. Remove remaining shared non-artifact state

Shared compiler-private state must either die or become real artifacts.

Targets:

- [language/compiler/src/compile/compiler.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/compiler.rs)
- `language/compiler/src/resolve/dependency`
- any registry helper forest that does not reduce to `publish`, `get`, `invalidate`, `dependency`, or `set_dependency`

Required end state:

- `module_binding_tables` is deleted or artifactized
- `import_locks` is deleted or proven to be true scheduler state
- compiler-local resolve caches are either local, deleted, or artifactized
- no semi-ambient compiler-private shared semantic caches remain

### 5. Keep persistence purely as persistence

Loaded workspace-index state must remain persistence metadata only.

Targets:

- [language/workspace/src/session/session.rs](/Users/florian/symbol/destack-4/language/workspace/src/session/session.rs)
- [language/workspace/src/workspace/index.rs](/Users/florian/symbol/destack-4/language/workspace/src/workspace/index.rs)

Required end state:

- loaded workspace-index data does not drift back into semantic truth, query truth, or scheduler truth
- `ArtifactStore` is the only persistence boundary for derived artifacts

### 6. Finish exact artifact contracts

Artifact payloads should contain only fields with real downstream consumers.

Targets:

- [language/workspace/src/program/dir.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/dir.rs)
- [language/workspace/src/program/mir.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/mir.rs)
- compiler phase producers and consumers under `language/compiler/src`
- query and linter consumers under `language/query/src` and `language/linter/src`
- codegen consumers under `language/codegen_*`

Required end state:

- every `Dir*` field has an actual downstream consumer
- every `Mir*` field has an actual downstream consumer
- emitted artifact families are exact and explicit
- no copied-forward artifact surface remains by habit alone

### 7. Re-check invalidation

Invalidation should stay version-first and dependency-first.

Targets:

- [language/workspace/src/program/invalidate.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/invalidate.rs)

Required end state:

- version bumps first
- dependency-stamp invalidation first
- eager physical deletion only when materially useful

### 8. Verify the final model

Before calling the architecture done, verify all of these:

- every long-lived mutable field is classified as input truth, artifact truth, scheduler state, or persistence metadata
- every remaining shared state holder outside those classes is deleted or justified
- every artifact family has one exact key and one exact dependency story
- emitted artifacts follow the same dependency and availability semantics as semantic artifacts
- no phase mutates shared published truth in place
- no helper layer reintroduces broad mutable state bags
- full compiler and spec gates are green

## Supporting Detail

Everything below this point is supporting detail, current-state notes, historical context, or narrower subplans.
If any of it disagrees with the canonical architecture or ordered transition plan above, the top sections win.

## Dir Artifact Model

The final `Dir*` model is:

- exact immutable phase artifacts
- flat fields on each `Dir*`
- no nested prior `Dir*`
- no generic `ModuleDir`
- no generic `ModuleDirBuilder`
- no generic `Dir*State`
- plain local mutable variables inside each phase by default
- unchanged fields reused with `Arc`
- mutated fields cloned locally and republished

This is a structural-sharing snapshot pipeline, not an inheritance chain and not a builder hierarchy.

### Construction rule

Each phase should follow this pattern:

1. read the prior committed artifact
2. clone only the fields this phase mutates into plain locals
3. keep unchanged fields as reused `Arc` fields
4. run the phase over those locals
5. publish the next exact `Dir*`

If a helper boundary becomes awkward, the first choice is to split the helper into smaller coherent operations.
The second choice is a tiny private local helper struct for one phase only.
The final design should not grow public `Dir*Builder`, `Dir*State`, `DirSurface`, `DirSemantics`, or similar cross-cutting nouns.

### Field carry-forward rule

Repeated fields across `Dir*` do not justify shared sub-structs by default.
Keep the `Dir*` payloads flat unless a sub-struct is clearly:

- semantically real
- lifecycle-coherent
- mutated together
- consumed together

Right now, broad sub-structs like `DirSurface`, `DirSyntax`, `DirSemantics`, `DirHeader`, and `DirNamespace` are not considered part of the target architecture.

### Current target field sets

The intended `Dir*` contracts are:

- `DirBase`
  - `tree`
  - `symbols`
  - `types`
  - `roots`
  - `anchor_node`
  - `namespace_symbol`
  - `namespace_scope`
  - `global_augmentation_scope`
  - `default_symbol`
  - `export_assignment_symbol`
  - `namespace_exports`
  - `module_bindings`

- `DirPrepared`
  - all still-needed `DirBase` fields
  - `import_meta`
  - `export_assignment`
  - `module_binding_exports`
  - `imported_modules`
  - `exported_symbols`

- `DirResolved`
  - all still-needed `DirPrepared` fields
  - resolved `tree`
  - resolved `symbols`
  - resolved `imported_modules`
  - resolved `exported_symbols`
  - resolved `module_binding_exports`
  - resolved `export_assignment`

- `DirDeclared`
  - all still-needed `DirResolved` fields
  - declared `types`
  - `captures`

- `DirInterface`
  - all still-needed `DirDeclared` fields
  - interface-converged `types`

- `DirAnalyzed`
  - all still-needed downstream fields
  - analyzed `types`
  - analyzed `captures`

- `DirElaborated`
  - all still-needed downstream fields
  - elaborated `tree`
  - elaborated `symbols`
  - elaborated `types`

- `DirPatched`
  - all still-needed downstream fields
  - patched `tree`

### Local mutation matrix

The current intended mutable local phase state is:

- base:
  - `tree`
  - `symbols`
  - `types`
  - `roots`
  - bind-time export scaffolding

- prepared:
  - `tree`
  - `symbols`
  - `roots`
  - export and import tables

- resolved:
  - `tree`
  - `symbols`
  - import and export tables

- declared:
  - `symbols`
  - `types`
  - `captures`

- interface:
  - `types`

- analyzed:
  - `types`
  - `captures`

- elaborated:
  - `tree`
  - `symbols`
  - `types`

- patched:
  - `tree`

This matrix is the standard for slice 5.
If a phase grows more mutable shared state than this, it needs a stronger justification than "the argument list got long".

## Current State After Slices 1 Through 3

This plan began as a forward-looking design document.
It now also needs to serve as an accurate current-state record.
Several major slices are already landed.
This section exists so the rest of the plan is read in the right temporal context.

### Landed slices

The following slices are already landed on the branch history.

#### Slice 1: semantic artifacts vs build outputs

Landed commit:
- `b3bc51f481` `refactor(language/workspace): separate semantic artifacts from build outputs`

What this accomplished:
- output-side `Artifact*` naming was removed
- `OutputRegistry` and `OutputStore` vocabulary landed
- semantic `Artifact*` vocabulary was freed and introduced

#### Slice 2: artifact-driven scheduler

Landed commit:
- `618b251309` `refactor(language/compiler): invert task dependency model to be artifact driven`

What this accomplished:
- `TaskDependency` was removed from semantic scheduling
- `AnalyzeDependencyStage` was removed
- the scheduler now runs on `BuildKey` and `BuildRequirement`
- `LintTask` and compiler-owned lint scheduling were removed
- `LanguageEnvironment`, `IntrinsicEnvironment`, `LibEnvironment`, and `GlobalEnvironment` were introduced
- `DirPrepared`, `DirPatched`, and `MirOptimized` were introduced

#### Slice 2 rebase fallout fix

Landed commit:
- `1b33efda84` `fix(language/compiler): align MIR, lower, and optimize after rebase`

This was not a new architectural slice.
It was a post-rebase cleanup for MIR parser drift, lower snapshot drift, and optimize expectation drift after rebasing onto main.

#### Slice 3: ownership cut

Landed commits:
- `77f6c9bb3d` `refactor(language/workspace): remove module-owned derived compiler state`
- `e95c0c5178` `refactor(language/compiler): move compiler builds onto artifact-backed state`
- `e41379e6df` `refactor(language): switch runtime and tooling consumers to artifacts`

Separate related cleanup:
- `e4dc71947f` or rebased equivalent `7d75936121` `fix(language/query): remove borrow-scope warning hacks`

What slice 3 accomplished:
- modules no longer own derived DIR, MIR, or comptime truth
- compiler producers and consumers are artifact-backed
- runtime, query, linter, service, daemon, CLI, and codegen now read derived semantic state from artifacts
- the old compiler workspace bridge file is gone
- the only remaining ownership bridge is centralized in `language/compiler/src/compile/frame.rs`

### Current green baseline

The last clean baseline on the slice-3 tree was:

- `cargo test -p destack_compiler --lib -- --test-threads=1`
  - `2018 passed`
  - `0 failed`
  - `9 ignored`

- `cargo test -p destack_test --test specification -- --test-threads=1`
  - `2475 passed`
  - `138 skipped`
  - `KNOWN FAILURES: 138`

This is the baseline that should be protected while continuing on the remaining publication and slice-4 work.

### Current unresolved issue

The original runtime-generator crash is fixed.
It exposed multiple slice-3 fallout bugs rather than one isolated failure.

What is now fixed:
- committed published type surfaces fail loudly if they still contain `Type::Unevaluated`
- zero-progress ambient namespace lookup no longer synthesizes bogus owner-qualified member paths
- selected-lib prepared-module requirements were moved out of resolver inner loops and up to the resolve task boundary
- selected-lib global symbols now resolve before injected prelude shims, so active lib `Promise` and `Symbol` surfaces win over core fallback shims
- declaration-backed member symbols now publish their declared value types, so consumers do not have to recover field types indirectly from object surfaces
- parenthesized spread type expressions now materialize as singleton rest tuples, which fixes exported rest-tuple newtype publication in builtin core
- core `Symbol` and `Promise` globals are now exposed through exported core surface and the prelude instead of relying on ambient luck
- commit-time unevaluated publication checks now reuse the shared compiler type-containment query instead of keeping a second bespoke walk just for this boundary
- runtime symbol lowering explicitly requires remote `DirPatched` artifacts when it crosses module boundaries
- binding catalog discovery now uses committed symbol decorator membership plus payload extraction from the actual resolved `@binding(...)` annotation, instead of comparing raw decorator symbol identity across artifact boundaries or name-matching decorators loosely
- runtime binding type lowering now keeps `Array<string>` as `Array<string>` and only lowers `Slice<string>` to `StringSlice`

Current verified state:
- `cargo run -p destack_runtime --features generate_bindings --bin generate-bindings` succeeds
- `cargo check -p destack_runtime --features generate_bindings` succeeds
- focused generator regressions for binding catalog extraction and exported type extraction now pass under `--features generate_bindings`
- `CARGO_INCREMENTAL=0 cargo test -p destack_compiler --lib -- --test-threads=1` passes with `2018 passed`, `0 failed`, `9 ignored`
- `CARGO_INCREMENTAL=0 cargo test -p destack_test --test specification -- --test-threads=1` passes with `2475 passed`, `138 skipped`, `0 unexpected failures`

Current remaining design note:
- generated ABI enums now follow committed semantic enum backing widths, which currently means many platform enums lower as the default integer width rather than the old compact range-based widths
- the runtime now bridges one handwritten diagnostic-to-ABI enum mapping by numeric discriminant instead of assuming equal Rust repr size
- if compact ABI enum widths are still desired, that needs an explicit source and compiler contract such as real `@repr(...)` support, not another generator-local inference pass

The important architectural point is unchanged:
- consumers should not repair bad compiler output
- consumers may depend on the right artifacts for the data they actually need
- committed semantic types and source-level binding annotations do not necessarily belong to the same final artifact, but that split must be explicit and stable

### Current uncommitted work

There is currently uncommitted work in the tree for slice-3 publication hardening and runtime-generator fallout cleanup.
It should be reviewed as in-progress, not as settled architecture.

Current modified files at the time of this plan update include work in:
- `language/compiler/src/analyze/commit/materialize.rs`
- `language/compiler/src/analyze/declare/collect/declaration.rs`
- `language/compiler/src/analyze/commit/pipeline.rs`
- `language/compiler/src/analyze/common/canonical.rs`
- `language/compiler/src/analyze/infer/tests/module.rs`
- `language/compiler/src/analyze/infer/tests/harness.rs`
- `language/compiler/src/analyze/declare/type/expression.rs`
- `language/builtin/core/control/result.ds`
- `language/builtin/core/control/index.ds`
- `language/builtin/core/prelude.ds`
- `language/compiler/src/resolve/binding/path.rs`
- `language/runtime/src/generate/analyze/collect.rs`
- `language/runtime/src/generate/analyze/types.rs`
- `language/runtime/src/generate/generator.rs`
- and a few adjacent fallout files

That work is directionally right in two important ways:
- runtime no longer reconstructs struct field types by matching declaration names against object fields
- compiler publication now owns the field-symbol and committed-surface contract directly

The remaining cleanup target is narrower:
- keep the runtime strict
- do not reintroduce consumer-side semantic repair
- decide whether ABI enum backing width should stay semantic-by-default or become an explicit source-level contract

### Current central transitional bridge

The biggest remaining sanctioned bridge is:
- `language/compiler/src/compile/frame.rs`

This file centralizes:
- `CURRENT_BUILD_FRAME`
- transient whole-`ModuleDir` rebuilds from committed artifacts
- `shared`, `private`, and `active` in-flight visibility policy

This is not final architecture.
It is the explicit slice-3 bridge.
It exists because:
- module-owned derived truth is gone
- but some builders still mutate whole `ModuleDir` and can yield/resume

The bridge is acceptable only because:
- it is centralized
- it is explicitly transitional
- it is tracked for deletion in slice 4

## Historical Ledger

This section preserves a compact chronology so the later action items are read in context.

### Pre-reset phase

The workstream originally resumed on top of a large in-progress state-model refactor.
That earlier attempt already had:
- movement away from module-owned state
- an `ArtifactRegistry` direction
- cleaner dependency thinking

But it still had major old-model seams:
- session-owned semantic fallback reads
- interface-component currentness problems
- tooling and formatting code reading raw registries
- scheduler logic still driven by old task/phase concepts

### Early architecture corrections

The first major settled points were:
- `Session` must not be a semantic owner
- `Program` is the semantic world
- builtins should not own mutable per-profile semantic state
- query/tooling should not discover semantic truth ambiently

This led to:
- `ModuleView` replacing `QueryContext`
- later extraction of `language/query`
- later move of `language/lsp`

### The failed broad attempt and reset

An earlier broad attempt was intentionally backed out because it had drifted too far from green.
That taught an important process lesson:
- keep architectural direction
- but re-land in smaller green slices

This reset was the foundation for the current slice structure.

### Central design pass

Before restarting the compiler core changes, the architecture was locked down explicitly:
- artifacts are semantic truth
- outputs are build products
- tasks are execution only
- build vocabulary is execution-only vocabulary
- modules own input state only
- caches hydrate and persist artifacts

This design pass also rejected:
- `BuildProduct`
- task-status-as-truth
- runtime-local semantic fallback logic
- keeping semantic truth on `Session`

### Structural cleanup before core compiler work

Several non-core but worthwhile cleanup slices landed before the major compiler slices:
- `language/query` extraction
- `language/lsp` move
- `language/base` to `language/core` rename

These were intentionally done early to reduce drift across other workstreams.

### Slice 1 recap

Slice 1 was mostly naming and ownership vocabulary.
Its main job was to free the `Artifact*` namespace for semantic products by renaming output-side concepts.

### Slice 2 recap

Slice 2 was the scheduler inversion:
- tasks stopped being the semantic dependency language
- `BuildKey` and `BuildRequirement` became the execution control plane
- environments and new DIR boundaries landed

This is the slice that made the current architecture possible.

### Slice 3 recap

Slice 3 removed module-owned derived truth.
This was the point where:
- `module.dir(profile)` stopped being semantic truth
- module-owned MIR/comptime state disappeared
- downstream tooling switched to artifacts

The cost of finishing slice 3 was the introduction of the centralized transient-frame bridge.
That is why slice 4 now exists as a focused follow-up rather than pretending slice 3 was the final builder architecture.

## Clarified Temporary vs Final Vocabulary

This section exists because some nouns in the current tree are final and some are explicit bridges.
The two must not be confused.

### Final nouns

These should remain:
- a single published product registry model
- a single published product key model
- a single published product dependency model
- `ArtifactRegistry`
- `ArtifactKey`
- `ArtifactDependency`
- `ArtifactRequirement`
- `Task`

Current implementation note:

- `ArtifactRegistry` and `OutputRegistry` still exist in the tree
- `ArtifactKey` and `OutputKey` still exist in the tree
- `ArtifactStore` and `OutputStore` still exist in the tree

These should be treated as implementation names, not as proof that two distinct architectural systems are desired.

### Explicitly bridge-only nouns

These were bridge-only nouns during the earlier slices.
They should not reappear:
- `CURRENT_BUILD_FRAME`
- `best_available_*`
- broad whole-`ModuleDir` transient rebuild helpers
- any ambient “current build” access outside the centralized frame layer

### Final product nouns

Published product families:
- `LanguageEnvironment`
- `IntrinsicEnvironment`
- `LibEnvironment`
- `GlobalEnvironment`
- `Ast`
- `DirBase`
- `DirPrepared`
- `DirResolved`
- `DirDeclared`
- `DirInterface`
- `DirAnalyzed`
- `DirElaborated`
- `DirPatched`
- `MirBase`
- `MirOptimized`
- emitted module and package artifact families

### Final execution nouns

Execution-only:
- `ArtifactRequirement`
- `Task`

These are not semantic products.
They describe the runtime machinery that realizes published artifacts.

## Current Non-Pristine Surfaces

This section names the remaining ugly-but-tracked parts directly.

### `language/workspace/src/program/dir.rs`

The generic mutable DIR bags are gone.
The remaining work is exact `Dir*` payload slimming.
The final target is flat immutable phase artifacts with plain local mutable tables inside each phase.
That means:
- no long-lived generic mutable DIR builder noun
- no generic `structure` or `surface` taxonomy layered across the whole compiler
- no shared mutable phase state
- phase products named by phase and containing only the fields that phase actually publishes
- no copied-forward fields without a real downstream consumer

### `language/workspace/src/workspace/index.rs`

`WorkspaceIndexSnapshot` is now only the persistence payload.
Loaded workspace-index file and module version seeds live directly on `Session`.
That boundary should stay persistence-only and must not regain semantic or query truth.

### `language/compiler/src/compile/compiler.rs`

The remaining compiler-owned shared state is now small, which makes the residue more suspicious rather than less.
`module_binding_tables`, `import_locks`, and any future shared compiler cache should be treated as guilty until deleted, artifactized, or proven to be true scheduler state.

### `language/workspace/src/artifact/registry.rs` and `language/workspace/src/output/registry.rs`

The current artifact and output split is still broader than the final model.
The final architecture wants one unified published product system, with family taxonomy rather than two different kinds of truth.

### `language/compiler/src/compile/compiler.rs`

Compiler-local shared state is now mostly down to scheduler state, diagnostics, cache I/O, and import serialization.
That is close to the intended end state.
Any new shared compiler cache should be treated as guilty until it is either promoted to a real artifact or proven to be unavoidable execution state.

## Runtime Generator Follow-up Plan

This needs a dedicated subsection because it is now the most important unresolved correctness issue.

### Invariant to enforce

Downstream consumers of committed semantic surfaces must not observe `Type::Unevaluated` where a fully resolved type is expected.

### Current failure mode

The runtime generator fails on:
- `UnixReceiveAncillary.handles`

The failure says:
- a committed struct field binding type is still unevaluated

### Correct direction

- runtime must not reconstruct compiler semantics locally
- compiler must publish a consumer-ready committed field surface
- if an intermediate helper is needed, it belongs in compiler code and should be treated as a bridge, not a destination

### Required next step

Add a compiler regression against the actual builtin/platform field surface, not just the generic declaration-field tests.

That test should:
- load the relevant builtin module
- read the committed exported struct instance surface
- inspect the field type
- assert it is fully resolved and consumer-ready

Only after that should the runtime generator be considered fixed.

## Updated Slice 4 Focus

Slice 4 is now complete on the semantic compiler path.

What landed:

1. `CURRENT_BUILD_FRAME` is deleted.
2. `BuildProduct` is deleted.
3. signature-driven invalidation is deleted.
4. ambiguous `best_available_*` and `dir_snapshot` style semantic reads are deleted from the compiler path.
5. committed artifact requirements now drive availability and staleness directly.
6. `Ast` and `DirBase` truth are artifact-owned instead of being mirrored back into mutable module state.
7. consumer-facing committed publication is hardened so runtime and query-style consumers do not need local semantic repair.
8. the analyze bridge was pruned further:
   - `ModuleContext` is deleted
   - `ModuleTreeView` is deleted
   - extra local-vs-remote analyze read helper indirection is deleted
9. the DIR nouns now match reality:
   - the published artifact noun is `ModuleDir`
   - `ModuleDirData` is gone as a public semantic truth noun
   - the old broad mutable import-state wrappers are gone
10. the low-value analyze mini-view duplication was pruned:
   - `TreeSymbolTypeView` is deleted
   - static-key and enum-literal helpers now read explicit `profile`, `tree`, `symbols`, and `types`
   - the remaining tree-boundary helpers stay because they still encode real local-vs-remote read gating rather than duplicating one immutable table bundle
11. the remaining fake support nouns were deleted:
   - `DirReadBoundary` is deleted
   - `ResolveDirRef` is deleted
   - `GlobalSymbolTableKey` is deleted
   - `ProgramId` is deleted
   - `ProgramStamp` is deleted
12. the workspace state model is further reduced:
   - `ProgramIndex` is deleted
   - `ModuleState` is deleted
   - `Program` no longer owns a workspace-index state handle
   - module graphs are real artifact truth
   - workspace-index state is session-owned persistence state only

What slice 4 intentionally did not force:

- replacing the internal `Shared<T>` layer with plain `Arc<T>`

That no longer looked like a simplification win.
At this point it would mostly spread `Arc::make_mut` and `Arc::unwrap_or_clone` churn across call sites instead of deleting real architecture.

## Updated Slice 5 Focus

Slice 5 is the final essence pass.
It should delete everything that is not input identity, published truth, exact keys, exact requirements, minimal scheduler state, or cache hydration and persistence.

### Current slice-5 direction

The current tree now mostly follows the intended ownership model.
The remaining work is to finish the artifact-shape model.

What is now true:
- `ProgramIndex` is gone
- `ProgramResolveCache` is gone
- `ProgramWorkspaceCache` is gone
- `ModuleState` is gone
- `GlobalSymbolTableKey` is gone
- `ProgramId` and `ProgramStamp` are gone
- `Program` no longer owns workspace-index state
- `WorkspaceIndexState` is gone
- loaded workspace-index state now lives directly on `Session`
- `ModuleRegistry` no longer spreads mutable input metadata across parallel maps
- `WorkspaceIndexSnapshot` only persists file and module version stamps
- module graphs are published artifacts
- compiler-only interface component index caching is gone
- the old artifact registry `set_*` and `remove_*` forest is gone

The remaining live architecture is now close to the minimal essence:
- input identity and config live in registries such as `ModuleRegistry`, `PackageRegistry`, `ProfileRegistry`, and `TsConfigRegistry`
- published derived truth lives in `ArtifactRegistry`
- execution bookkeeping lives in the scheduler
- persistence and hydration state lives in session and workspace cache code

### Final slice-5 artifact model

The final artifact model should be simple and explicit.

Shared state:
- immutable phase artifacts only
- no generic shared semantic bag
- no generic shared mutable builder

Owned state:
- plain local mutable tables inside one phase task
- clone only the tables a phase actually mutates
- reuse unchanged `Arc` fields directly when publishing the next phase artifact

The target shape is:
- `DirBase`
- `DirPrepared`
- `DirResolved`
- `DirDeclared`
- `DirInterface`
- `DirAnalyzed`
- `DirElaborated`
- `DirPatched`
- `MirBase`
- `MirOptimized`

These are phase products, not abstractions over “kinds of state”.
Their fields should stay flat unless a sub-struct is obviously earned by the language or runtime model itself.
`Ast` can remain singular for now because parse currently publishes one syntax product, not a phase ladder.

Performance rule:
- unchanged published tables stay shared by cheap `Arc` clones
- changed tables are explicitly cloned once into local mutable values
- no `Arc::make_mut` on compiler phase state

Example pattern:
- read one previous immutable phase artifact
- clone only the fields that will actually be mutated
- pass plain `&mut` locals through the phase
- publish one new immutable phase artifact with reused `Arc`s for unchanged fields and fresh `Arc`s for changed fields

### Remaining slice-5 pressure

What still deserves pressure:
- keep compiler-local caches ephemeral and local to one operation unless they are promoted to real artifacts
- keep deleting convenience wrappers that do not encode a real ownership or execution boundary
- slim the exact `Dir*` payload contracts so each phase only carries the fields real downstream consumers still use
- delete generic mutable helper APIs like `tree_mut`, `symbols_mut`, and `tree_symbols_types_mut`
- keep phase mutation as plain local owned tables unless one named producer context is truly earned by the phase itself

Current exact state:
- `ModuleDirBuilder` is gone
- `ResolvedDirState` is gone
- compiler producers now publish exact `Dir*` artifacts directly from phase locals
- `import_meta` is no longer part of the shared `Dir*` payloads
- `module_bindings` now stop at the base and prepared artifacts where they are actually needed
- resolve-time linkage now stays on `DirResolved` instead of leaking through `DirDeclared` or `DirAnalyzed`
- `DirInterface` no longer carries declared-state baggage like captures or imported modules just to shuttle them into analyze
- query and linter read linkage data from `DirResolved` instead of bloating `DirAnalyzed`
- `DirElaborated` and `DirPatched` no longer carry the old resolve-time linkage surface or dead namespace/default/export-assignment baggage
- workspace and compiler test scaffolding clone explicit test snapshots instead of depending on shared mutable builder state

What this slice should now force:
- phase-named immutable artifacts
- flat fields
- local mutation only
- no generic DIR bag as the semantic publication model
- no generic MIR bag as the semantic publication model

### Current blocker outside slice 5

Slice 5 is no longer blocked on compiler or workspace ownership cleanup.
The current blocker to full-system green is a VM regression in managed aggregate execution that predates this slice and reproduces with identical VM interpreter code on `main`.

What is failing:
- class allocation and field access execution
- closure capture execution
- interface call execution

What is currently understood:
- the VM still has an inconsistent representation boundary between packed-value managed aggregates and raw-byte managed aggregates
- whole-value managed typed load and store still project aggregate types as though they were field or element loads
- raw aggregate byte encoding and decoding is incomplete for aggregate layouts

What this means for slice 5:
- close out the architecture slice on compiler and workspace shape
- track the VM fix separately as the current green-gate blocker
- do not contort compiler or workspace architecture to paper over a VM runtime bug

## Completion Status For The Deletion Checklist

This section mirrors the older deletion checklist, but marks what is already done on the current branch history.

### Completed already

- [x] Rename generated `Artifact*` output types to `Output*`
- [x] Remove semantic use of `TaskDependency`
- [x] Remove `TaskDependencyError`
- [x] Remove scheduler logic that scans task status for semantic satisfaction
- [x] Remove `do_require_task_internal_only`
- [x] Remove phase narration `require_*` APIs as semantic task dependencies
- [x] Remove `AnalyzeDependencyStage`
- [x] Remove standalone validate boundary as a public scheduler boundary
- [x] Remove solve, commit, capture task facades from semantic scheduling
- [x] Remove module-owned mutable derived phase state
- [x] Remove ambient inner logic `*_maybe` artifact lookups as control flow in the old module-owned sense

### Not done yet

- [ ] Remove broad downstream invalidation as a correctness mechanism
- [ ] Remove any remaining compatibility aliases or reexports created during the rewrite
- [ ] Remove module input version updates caused by semantic downstream changes where they still exist
- [ ] Remove scheduler or cache logic that treats rerun as semantic change without digest change
- [ ] Remove transitive semantic dependency blobs where direct artifact dependencies suffice

## Explicit Cross-Document Consistency Notes

This plan and `HANDOFF.md` should agree on the following.

1. Slices 1 through 3 are landed.
2. Slice 2 rebase fallout fix is separate and landed.
3. The runtime-generator publication issue is fixed and should stay fixed at the compiler publication boundary.
4. `CURRENT_BUILD_FRAME` was transitional and is now gone.
5. Runtime-local semantic fallback and runtime-local canonicalization are rejected.
6. `Build*` is execution-only vocabulary.
7. `BuildProduct` does not exist.
8. `HANDOFF.md` contains the detailed chronology and context.
9. This plan contains the forward-looking architecture and tracked debt.

If either document drifts from those points, update both.

## Final Nouns

The core architectural nouns are:

- `Program`
- `Module`
- `Session`
- `CacheStore`
- `ArtifactRegistry`
- `ArtifactStore`
- `ArtifactKey`
- `ArtifactDependency`
- `ArtifactRequirement`
- `Task`

Everything else must justify itself as either:

- a real reusable published product
- input state
- or a local implementation detail

The final architecture should not need `BuildKey`, `BuildRequirement`, `OutputRegistry`, `OutputKey`, or `OutputStore`.
All derived products, including emitted files and linked bundles, should be artifacts.
The only non-artifact leaves are versioned input facts in the input registries.

Current implementation note:

- the tree still contains `ArtifactRegistry`, `OutputRegistry`, `ArtifactKey`, `OutputKey`, `ArtifactStore`, and `OutputStore`
- the target architecture is one artifact graph
- emitted outputs should collapse into artifact families
- `Build*` should collapse into artifact-facing scheduler vocabulary

## Ownership

`Program` owns the live input world and the authoritative published artifact state.
`Session` owns service lifetime, frontier sharing, and external resources only.

Modules own input state only.
Modules do not own current DIR, infer state, analyzed state, elaborated state, comptime state, MIR state, or any other derived semantic phase state.

Builders own transient mutable state while a task is executing.
That mutable state dies when the task completes.
No builder may publish its result by mutating `Module`.

`CacheStore` is the shared low-level cached byte store.
`ArtifactStore` is the typed artifact persistence layer over `CacheStore`.
`FileSystem` remains the abstraction for source files and user-visible emitted files, not for cached artifact persistence.

## Semantic Truth

Artifacts are truth.
Dependencies explain when artifacts are valid.
Task status is never semantic truth.
Phase completion is never semantic truth.
Diagnostics are not semantic truth.

The dependency graph is artifact to artifact only.
There is no second output graph and no separate semantic task graph.
Inputs remain versioned facts outside the artifact graph.

## Final Artifact Set

The published artifact families are:

- `LanguageEnvironment(profile)`
- `IntrinsicEnvironment(profile)`
- `LibEnvironment(profile)`
- `Ast(module)`
- `DirBase(module)`
- `DirPrepared(module, profile)`
- `DirResolved(module, profile)`
- `DirDeclared(module, profile)`
- `DirInterface(module, profile)`
- `DirAnalyzed(module, profile)`
- `DirElaborated(module, profile)`
- `DirPatched(module, profile)`
- `MirBase(module, profile, target)`
- `MirOptimized(module, profile, target)`
- emitted module outputs
- linked package outputs
- emission records when they need to be persisted or scheduled as derived state

Generated JS, source maps, object files, wasm, bundles, linked binaries, and emitted files are artifacts.
They should live in `ArtifactRegistry`.

### Semantic artifact families

| Family | Key | Primary payload | Default persistence | Notes |
| --- | --- | --- | --- | --- |
| `LanguageEnvironment` | `ProfileId` | Language semantic environment | disk | Language items, prelude-facing builtin bindings, and other compiler-known language state for one profile |
| `IntrinsicEnvironment` | `ProfileId` | Intrinsic semantic environment | disk | Well-known intrinsic bindings derived from builtin declaration decorators for one profile |
| `LibEnvironment` | `ProfileId` | Lib semantic environment | disk | Selected lib modules, declared lib symbols, merge sources, and well-known symbol state for one profile |
| `Ast` | `ModuleId` | Parsed syntax tree | disk | Pure syntax and source-derived metadata |
| `DirBase` | `ModuleId` | Bound and desugared base DIR | disk | Shared across profiles |
| `DirPrepared` | `(ModuleId, ProfileId)` | Prepared profile DIR state | disk | Local profile shaping before cross-module resolution |
| `DirResolved` | `(ModuleId, ProfileId)` | Resolved DIR state | disk | First profile-specific semantic compiler product |
| `DirDeclared` | `(ModuleId, ProfileId)` | Declared semantic surface | disk | Upstream dependency source for interface and analyze |
| `DirInterface` | `(ModuleId, ProfileId)` | Published module interface | disk | Published per module, computed by internal SCC fixed point when needed |
| `DirAnalyzed` | `(ModuleId, ProfileId)` | Final analyzed and validated DIR | disk | Validation is part of this artifact |
| `DirElaborated` | `(ModuleId, ProfileId)` | Elaborated canonical DIR | disk | Real boundary between analyze and comptime |
| `DirPatched` | `(ModuleId, ProfileId)` | Post-comptime DIR | disk | Elaborated DIR after comptime patching |
| `MirBase` | `(ModuleId, ProfileId, TargetId)` | Baseline MIR | disk | Input to optimization and some direct backends |
| `MirOptimized` | `(ModuleId, ProfileId, TargetId)` | Optimized MIR | disk | Canonical MIR input to output generation |

Every family is a semantic compiler product.
Persistence is a per-family policy.
The default bias is to persist every non-trivial family, but persistence policy is still distinct from semantic identity.

### Emitted artifact families

| Family | Key | Primary payload | Default persistence | Notes |
| --- | --- | --- | --- | --- |
| `GeneratedArtifact` | emitted artifact key | emitted bytes or structured output | disk | Generated file or blob such as JS, d.ts, source map, object code, or wasm output |
| `LinkedArtifact` | emitted artifact key | linked or bundled output | disk | Linked or bundled output before final emit |
| `EmissionArtifact` | emitted artifact key | emission record | memory | Write intent and bookkeeping for emitted files |

The exact emitted artifact family split can evolve with backend needs.
The key rule is that emitted products are also artifacts.

## Final Naming

The stable IR artifact vocabulary is:

- `LanguageEnvironment`
- `IntrinsicEnvironment`
- `LibEnvironment`
- `Ast`
- `DirBase`
- `DirResolved`
- `DirDeclared`
- `DirInterface`
- `DirAnalyzed`
- `DirElaborated`
- `DirPatched`
- `Mir`
- `MirOptimized`

This is the long-term naming standard.
Names like `DirResolution`, `DirDeclare`, `InterfaceComponent`, `ExecutePatch`, and similar transitional terms should disappear.

## Artifact Payload Shape

Published artifact values are immutable.
They are not `RwLock<ModuleDir>`.
They are frozen values assembled from shared immutable substructures.

The intended shape is:

- shared tree data
- shared symbol and scope data
- shared type data
- shared capture data
- shared export and interface data
- small scalar metadata

Phase transitions use structural sharing.
Later artifacts reuse the unchanged parts of earlier artifacts.
We do not deep-clone full DIR trees.

The implementation model is:

- mutable transient builders during execution
- immutable frozen artifact values on commit

### Phase payload decomposition

The final semantic model should not publish one giant universal `ModuleDir` or `ModuleMir` payload.
Different artifact families should use payload types named after the artifact family itself.
The payload fields should stay flat.
Do not introduce cross-cutting wrapper taxonomies like `structure`, `surface`, or `semantics`.

The intended artifact payloads are:

| Artifact | Payload shape | Notes |
| --- | --- | --- |
| `LanguageEnvironment` | `Arc<LanguageEnvironmentData>` | Profile-scoped language semantic environment derived from builtin module interfaces |
| `IntrinsicEnvironment` | `Arc<IntrinsicEnvironmentData>` | Profile-scoped intrinsic binding environment derived from builtin declared decorator surface |
| `LibEnvironment` | `Arc<LibEnvironmentData>` | Profile-scoped selected lib environment derived from lib module resolution |
| `Ast` | `Arc<Ast>` | Pure syntax |
| `DirBase` | `Arc<DirBase>` | Base tree, symbols, roots, anchor, and module namespace ids |
| `DirPrepared` | `Arc<DirPrepared>` | `DirBase` plus profile-local setup needed before cross-module resolution |
| `DirResolved` | `Arc<DirResolved>` | `DirPrepared` plus resolved import and export state |
| `DirDeclared` | `Arc<DirDeclared>` | Declared semantic tables over the resolved tree and symbol space |
| `DirInterface` | `Arc<DirInterface>` | Compact published module interface plus only the linkage surface still needed for interface consumers |
| `DirAnalyzed` | `Arc<DirAnalyzed>` | Final analyzed semantic tables, not a linkage snapshot |
| `DirElaborated` | `Arc<DirElaborated>` | Elaborated tree, symbols, types, captures, roots, and minimal module anchors |
| `DirPatched` | `Arc<DirPatched>` | Post-comptime tree plus the minimal elaborated carry-forward state |
| `MirBase` | `Arc<MirBase>` | Baseline MIR payload |
| `MirOptimized` | `Arc<MirOptimized>` | Optimized MIR payload |

This is intentionally not one universal `ModuleDir` or `ModuleMir` artifact type.
The final model should let `DirInterface` be much smaller than `DirAnalyzed`, and `MirOptimized` be a distinct published product from `MirBase`.

### Phase-local mutation

Every phase should mutate plain local owned tables unless a narrower named producer context is clearly earned.
The default shape is:

- read one immutable upstream artifact
- clone only the fields the phase will mutate
- thread plain `&mut` locals through the phase
- publish one immutable downstream artifact

Do not stabilize one generic mutable `ModuleDirBuilder` or `ModuleMirBuilder`.
If a phase ends up with a named producer context, it must be phase-specific and smaller than the old generic bag.

## Build Graph And Task Model

`Task` stays as an execution noun.
Its meaning is:

- realize one `ArtifactKey`
- for one captured dependency attempt

Tasks are not semantic phase boundaries.
Tasks are not semantic truth.

There should be at most one in-flight task per `ArtifactKey`.
Waiters attach to artifact keys, not to phase names.

The singleflight unit is the artifact key.
The satisfaction unit is artifact key plus expected dependency.
That means:

- only one task builds `K` at a time
- a task captures one dependency attempt for `K`
- registry satisfaction always checks `K@D`, not just `K`

### Task shape

The final task shape should be execution-oriented and build-native.

Conceptually:

```rust
struct Task {
    key: ArtifactKey,
    dependency: ArtifactDependency,
}
```

What we should not keep is a semantic task taxonomy that duplicates artifact families and subphases.

`TaskPhase` may remain for telemetry and progress reporting.
It should be derived from `ArtifactKey`.
It should not be the semantic dependency language.

## Requirement Model

Semantic code expresses artifact requirements.
It does not schedule tasks directly.

The semantic operations are:

- compute expected dependency
- require artifact
- read artifact at dependency
- build artifact value
- hydrate artifact
- persist artifact

The engine operations are:

- enqueue task
- claim task
- resume task
- commit artifact
- wake waiters
- persist artifact

Emitted artifacts follow the same operations.
The scheduler should drive one artifact graph, not separate semantic and output systems.

### Profile environments

Some semantic compiler products are profile-scoped environments rather than module IR snapshots.
These should not be forced into the `Dir*` family.

The final model should publish:

- `LanguageEnvironment(profile)`
- `IntrinsicEnvironment(profile)`
- `LibEnvironment(profile)`

`LanguageEnvironment` is the compiler-known language environment for one profile.
It should include language items, prelude-facing builtin bindings, and other builtin semantic state needed during resolve and analyze.

`IntrinsicEnvironment` is the profile-scoped intrinsic binding environment.
It should include well-known intrinsic bindings derived from builtin declaration decorators.

`LibEnvironment` is the profile-selected library environment.
It should include selected lib modules, declared lib symbols, ambient symbol groups and sources, and well-known symbols.

These are semantic compiler products, not mutable caches on `Builtins`.
`Builtins` should become an immutable builtin catalog: builtin module ids, builtin lib metadata, and other input-side builtin data only.

The intended bootstrap order is:

- builtin module interfaces
- `LanguageEnvironment`
- builtin module declarations
- `IntrinsicEnvironment`
- lib module interfaces
- `LibEnvironment`
- ordinary module products

That keeps the dependency graph explicit and acyclic.

Requirement shape should support:

- a single requirement
- an all-of requirement set
- an any-of requirement set where a real execution need exists

The scheduler-level dependency language should therefore be:

- one `ArtifactRequirement`
- or one conjunction of requirements
- or one disjunction of requirements where required

### Requirement API

The semantic requirement API should be artifact-shaped and explicit.

The core internal operations should be close to:

- `expected_artifact_dependency(&self, key) -> ArtifactDependency`
- `artifact_maybe_at(&self, key, dependency) -> Option<ArtifactValue>`
- `require_artifact(&self, key, dependency) -> Result<(), ArtifactRequirementError>`
- `collect_requirement(&self, collector, result)`

Typed wrappers should exist per family.
Examples:

- `require_ast(module)`
- `require_dir_base(module)`
- `require_dir_resolved(module, profile)`
- `require_dir_declared(module, profile)`
- `require_dir_interface(module, profile)`
- `require_dir_analyzed(module, profile)`
- `require_dir_elaborated(module, profile)`
- `require_dir_comptime(module, profile)`
- `require_mir_base(module, profile, target)`
- `require_mir_optimized(module, profile, target)`
- `require_generated_artifact(scope, target, kind)`
- `require_linked_artifact(scope, target, kind)`

These replace the old phase-shaped `require_import_*`, `require_resolve_*`, `require_analyze_*`, and similar facades.

### Read API

Read APIs should also be artifact-shaped.

Typed read helpers should be close to:

- `ast_at(module, dependency)`
- `dir_base_at(module, dependency)`
- `dir_resolved_at(module, profile, dependency)`
- `dir_declared_at(module, profile, dependency)`
- `dir_interface_at(module, profile, dependency)`
- `dir_analyzed_at(module, profile, dependency)`
- `dir_elaborated_at(module, profile, dependency)`
- `dir_comptime_at(module, profile, dependency)`
- `mir_at(module, profile, target, dependency)`

Higher-level convenience APIs may compute the expected dependency first.
Inner logic should consume already-required artifacts instead of doing ambient `*_maybe` control-flow reads.

### Requirement collector

The current `TaskResultCollector` concept should survive in simplified form as an artifact requirement collector.

Its job is:

- collect yielded artifact requirements from multiple sub-operations
- coalesce them into one conjunction
- let hard errors pass through unchanged

It should not know about task status or task ids.

## Producer Contract

Each artifact family has one producer recipe.
That producer can only:

- yield artifact requirements
- complete with the promised artifact value
- fail fatally because it cannot publish the promised artifact

If the producer published the promised artifact, it succeeded.
User-facing diagnostics do not count as producer failure.

Publication is owned by the engine.
Normal semantic code should not call a broad `publish_artifact(...)` API.

### Producer interface

The producer side should be explicit per artifact family.
Conceptually:

```rust
trait ArtifactProducer {
    type Value;

    fn build(
        &self,
        key: &ArtifactKey,
        dependency: &ArtifactDependency,
        context: &mut BuildContext,
    ) -> BuildOutcome<Self::Value>;
}
```

We do not need to encode this as a literal public trait if a concrete dispatch table is simpler.
The important part is the contract, not the abstraction mechanism.

## Scheduler Contract

The scheduler is operational only.
It should:

- deduplicate in-flight artifact builds
- run producers
- translate yielded artifact requirements into more work
- resume waiters when required artifacts become available
- discard stale completed work when dependencies drifted before commit

The scheduler must not:

- interpret diagnostics
- infer semantic truth from task status
- compensate for producers that returned `Err` after publishing usable artifacts
- encode phase-specific semantic policy

For `DirInterface`, the engine may commit a converged SCC batch atomically.
That is an internal commit strategy only.
The public artifact model remains per-module `DirInterface(module, profile)`.

### Internal scheduler API

The scheduler-facing compiler operations should be close to:

- `enqueue_task(key, dependency)`
- `run_ready_tasks()`
- `run_to_completion()`
- `run_until_artifact(key, dependency)`
- `wake_waiters(key)`
- `commit_artifact(key, dependency, value, digest)`
- `discard_stale_completion(key, dependency)`

The exact method names can vary, but the execution model should not.

### Compiler ingress API

Inside the compiler, orchestration code should drive work by requiring artifacts, not by requiring phase tasks.

Examples:

- compile a module to MIR by requiring `Mir(module, profile, target)`
- compile a module to optimized MIR by requiring `MirOptimized(module, profile, target)`
- type-check a module by requiring `DirAnalyzed(module, profile)`
- build an interface surface by requiring `DirInterface(module, profile)`

### External ingress API

Outside the compiler, callers should request end results in semantic terms.
Examples:

- compiler CLI path requests outputs or MIR
- query path requests `DirAnalyzed` or `DirInterface` views
- linter path requests semantic compiler products like `DirAnalyzed`, `DirPatched`, or `MirOptimized`
- lint is not part of the build graph and should not introduce `LintTask`
- emit path requests output artifacts

The engine may still expose blocking convenience calls like:

- `build_artifact(key)`
- `build_output(key)`
- `compile_to_completion()`

Those should be thin wrappers over the same artifact-driven scheduler, not a second execution model.

## Versioning, Digests, and Currentness

There are three different layers here.

### Input versions and stamps

Input state keeps mutable versions and stamps.
Examples include:

- file version
- module version
- package version
- profile or config stamp
- workspace level input stamps where needed

These are cheap mutable inputs used to compute expected artifact dependencies.

Input versions must remain input-only.
In particular, module input versions must not advance because imported semantic state changed.
Imported semantic change propagation must happen through artifact dependencies and digests, not by mutating module input versions.

### Artifact dependencies

Each published compiler product stores the exact `ArtifactDependency` it was built against.
That dependency is the currentness contract for that artifact.

### Artifact digests

Each published compiler product stores a stable `ArtifactDigest` of its semantic value.
Downstream artifacts depend on upstream digests.

We do not want a separate semantic compiler product version counter.
If an operational generation counter exists, it is debug or eviction metadata only.

Digest propagation should be red-green.
If an artifact is rebuilt and its digest is unchanged, downstream artifacts should not become semantically dirty just because the producer ran again.

## Interface Fixed Point

`DirInterface` is published per module.
SCC or component evaluation is an internal execution strategy only.

That means:

- the scheduler or producer may evaluate an interface SCC together
- once convergence is reached, it publishes `DirInterface(module, profile)` for each module in that SCC

There is no public `InterfaceComponent` artifact family in the final model.
Component planning is an internal compiler concern.

## Caching

The in-memory registry is authoritative.
Disk cache is only hydration and persistence.
There is no second semantic state path.

The final layering is:

- `FileSystem`: source files and user-visible emitted outputs
- `CacheStore`: shared cached byte substrate
- `ArtifactStore`: typed artifact persistence over `CacheStore`
- `ArtifactRegistry`: authoritative in-memory derived state

The cache flow is:

1. need artifact `K` at expected dependency `D`
2. check in-memory registry for `K@D`
3. if absent, try persistent hydration for `K@D`
4. if hydration succeeds, seed registry
5. otherwise schedule build
6. on successful commit, persist artifact record

Persistent cache records should store:

- artifact family
- artifact key
- dependency digest
- content digest
- schema version
- toolchain version
- relevant options hash
- serialized artifact value

The default bias is to cache every non-trivial published artifact.
Artifacts are more cacheable precisely because they are immutable and dependency keyed.

Not every artifact family must be persisted from day one.
Persistence policy is per-family.
All persisted artifacts must still go through `ArtifactStore`.

## Dependency Discipline

Artifact dependencies should be direct rather than transitive whenever possible.
Each producer should depend on the exact upstream artifact digests it actually consumed.

The model should avoid:

- transitive closure dependency blobs
- broad graph snapshots used as semantic proof without need
- scheduler history as a stand-in for semantic dependency

This is important for both performance and clarity.

## `.destack` Layout

The repository and user-level state split remains part of the final design.

- `~/.destack`: install-level and safely shareable global resources
- `repo/.destack`: workspace-local mutable state

Within the workspace-local root, semantic ownership should be reflected directly in the path layout.
The language semantic cache and stores should live under:

- `repo/.destack/language/workspace`
- `repo/.destack/language/compiler`
- `repo/.destack/language/runtime`

For this refactor, the relevant root is `repo/.destack/language/workspace`.
That is where the workspace-owned `CacheStore` root for artifacts should resolve by default.
`ArtifactStore` should then define typed namespaces within that root, rather than inventing separate top-level artifact and output domains.

## Invalidation

Correctness invalidation is lazy.

Input edits and configuration changes eagerly bump input versions and stamps.
Derived artifacts become stale because their stored dependency no longer matches the expected dependency.

Correctness must not depend on eager downstream clearing.
Eager deletion is allowed for cleanup and memory pressure only.

## Program and Module Structure

The final structure should move toward:

- input and identity state on `Module`
- all derived artifacts in `ArtifactRegistry`
- query and tooling reads through artifact ingress

`ModuleDir` must not remain the published semantic truth container.
The final model is immutable artifact payloads, not mutable module-owned DIR bags.

`ProgramIndex` should not survive as a junk-drawer noun.
Its current responsibilities should split into:

- input indexes on `Program`
- artifacts in `ArtifactRegistry`
- query-side caches in query code

We should introduce an explicit input-side graph or dependency index noun where needed instead of leaving that role implicit.
That graph should remain an input-side support structure, not a published semantic compiler product by default.

## Diagnostics

Diagnostics are produced during artifact builds.
They do not determine artifact validity.

The intended model is:

- diagnostics are tied to artifact build attempts
- diagnostics may be persisted alongside cache records when useful
- diagnostics are replaced atomically when a newer artifact build supersedes them
- diagnostics are never part of `ArtifactDigest`

## Supersession

The scheduler should support stale work finishing safely.

That means:

- stale in-flight tasks may complete and then be discarded
- new required work should not be blocked on obsolete dependency attempts
- supersession should be an explicit operational behavior, not an accidental side effect

This matters for daemon and editor use where input state changes while work is in flight.

## Views

Long-term consumers should not pass `Program` everywhere for inner logic.

The intended layering is:

- orchestration requires artifacts from the compiler
- inner logic consumes typed views over the resulting artifacts

This applies to query, lint, daemon, and other tooling code.

## Historical Backtest Against The Pre-Slice Codebase

This section records the major old-model conflicts that existed before slices 1 through 3 landed.
It is preserved for historical context and design justification.
It should not be read as claiming that all of these items are still live on the current branch.

### Task dependency as semantic dependency language

- [task.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/task.rs#L364)
- [task.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/task.rs#L499)
- [task.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/task.rs#L550)

At that point, `TaskDependency` and `TaskDependencyError` were still the semantic dependency language.
They were later replaced by `BuildRequirement`.

### Scheduler reasoned in task space

- [process.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/process.rs#L465)
- [process.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/process.rs#L518)
- [process.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/process.rs#L539)

At that point, the scheduler still checked readiness and failure by scanning task status trees.
That was later replaced with artifact/build-key satisfaction.

### Phase-shaped require APIs

- [compiler.rs](/Users/florian/symbol/destack-4/language/compiler/src/compile/compiler.rs#L484)
- many `require_*` helpers across `import`, `resolve`, `analyze`, `elaborate`, `execute`, `lower`, `generate`, and `link`

At that point, these APIs still expressed semantic dependencies in task and phase terms.
They were later replaced with artifact-shaped requirement helpers.

### Procedural stage gating

- [stage.rs](/Users/florian/symbol/destack-4/language/compiler/src/analyze/module/stage.rs#L6)

`AnalyzeDependencyStage` was the old procedural read model.
It was later deleted in favor of artifact boundary vocabulary.

### Standalone validate boundary

- [process.rs](/Users/florian/symbol/destack-4/language/compiler/src/analyze/validate/process.rs#L8)

Validation still existed as a fake public boundary.
In the final model, validation is part of `DirAnalyzed`.

### Generated output registry owns the `Artifact*` namespace

- [artifact.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/artifact.rs#L92)
- [artifact.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/artifact.rs#L242)
- [session.rs](/Users/florian/symbol/destack-4/language/workspace/src/session/session.rs#L41)

At that point, `ArtifactRegistry` was still actually a generated output store.
That namespace was later freed by renaming the output-side system to `OutputRegistry`.

### Modules still own mutable semantic state

- [dir.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/dir.rs#L14)
- [dir.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/dir.rs#L63)

At that point, `ModuleDir` was still a mutable bag of `RwLock` based semantic tables, including infer handoff state.
That ownership model was later removed from `Module`.

### ProgramIndex and signatures are still a second dependency system

- [index.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/index.rs#L18)
- [signature.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/signature.rs#L28)
- [signature.rs](/Users/florian/symbol/destack-4/language/workspace/src/program/signature.rs#L128)

`ModuleSignature` and `ModuleSignatureDigest` still need to disappear into artifact digests.
`ProgramIndex` still needs to stop being the general home for derived semantic truth.

## Original Green Slice Plan

This was the original slice plan before the first three slices landed.
It is preserved because the ordering and rationale are still useful.
The current state is recorded above, and the completion ledger below supersedes the unchecked historical checklist items.

Every slice was intended to stay green.
The gate was compiler unit tests and spec baseline, not just `cargo check`.

### Slice 1: free the semantic namespace and split outputs from semantic compiler products

Goals:

- rename current generated `Artifact*` types to `Output*`
- rename current generated `ArtifactRegistry` to `OutputRegistry`
- introduce `OutputStore`
- introduce the semantic compiler product vocabulary under `language/workspace`
- make the ownership rule explicit in code: modules are input only, not semantic state owners
- remove generated-output `Artifact*` naming entirely
- remove any compatibility aliases between `Artifact*` and `Output*`

Rules:

- no compatibility aliases from `Artifact*` to `Output*`
- no new module-owned semantic phase state

Primary deletions:

- generated-output `ArtifactRegistry`
- generated-output `ArtifactKey` and related `Artifact*` output names
- any mixed semantic and output naming in workspace or compiler code
- any module-version update paths that fold semantic downstream change into input versions

### Slice 2: rewrite the scheduler around artifact requirements

Goals:

- replace `TaskDependency` with `BuildRequirement`
- make yielded task state carry artifact requirements only
- key waiting and singleflight by artifact key
- make task failure mean only fatal no-publication failure
- support only single and all-of requirement shapes
- remove scheduler fallback error logic derived from task dependency trees
- remove semantic task-completion checks from the scheduler
- introduce `BuildKey`
- introduce `LanguageEnvironment`, `IntrinsicEnvironment`, `LibEnvironment`, and `DirPrepared`
- replace `AnalyzeDependencyStage` with artifact-aligned DIR read boundaries
- remove compiler-owned lint scheduling from the build graph

Rules:

- no scheduler interpretation of diagnostics
- no semantic task dependency graph

Primary deletions:

- `TaskDependency`
- `TaskDependencyError`
- yielded dependency trees over tasks
- dependency-tree fallback failure logic
- task-status-as-semantic-truth logic
- scheduler semantics based on rerun instead of digest change
- `LintTask`
- `TaskPhase::Lint`
- `AnalyzeDependencyStage`

### Slice 3: rewrite producers and evacuate derived state from modules

Goals:

- replace phase-shaped `require_*` calls with artifact-shaped requirement helpers
- fold validate into `DirAnalyzed`
- keep `DirElaborated`, `DirPatched`, `Mir`, and `MirOptimized` as real boundaries
- move all mutable derived state into transient builders
- stop storing current semantic phase state on `Module`
- remove fake public subphase boundaries
- remove ambient inner-logic artifact hunting and fallback reads
- start replacing signature and index assumptions that block the new producer model

Rules:

- no standalone validate boundary
- no fake solve, commit, capture task facades
- no producer publication by mutating input-owned objects
- do not fix regressions in old task or phase terms while this slice is in flight
- delete the temporary scheduler-side module workspace rehydration bridge added in slice 2
- delete the temporary resolve-side `ResolveState.active_tree` bridge added in slice 2
- task-owned transient build state is the only mutable derived DIR state
- in-flight reads may observe task-owned transient state when the requested read boundary is satisfied
- task-owned transient state is private by default and only becomes visible through the current task frame
- `current_active_dir_frame(...)` is the intended same-build read path
- `active_or_transient_dir_from_artifact(...)` and `with_transient_artifact_dir(...)` are the centralized artifact-to-transient builder bridge
- `CURRENT_BUILD_FRAME` is an explicit slice-3 bridge only, not a lasting architectural noun
- `best_available_*` helpers are explicit bridge-policy helpers only, not semantic truth APIs

Primary deletions:

- phase-shaped semantic `require_*` APIs
- `AnalyzeDependencyStage`
- standalone validate boundary
- solve, commit, and capture task facades
- module-owned current DIR, infer, analyzed, elaborated, comptime, and MIR state
- ad hoc module-workspace mirrors used as semantic truth
- ambient `*_maybe` control-flow reads in inner logic
- any signature or index usage kept only to prop up module-owned semantic state

Implementation order:

1. Builder ingress
   Replace scheduler-side workspace rehydration with explicit artifact-backed builder ingress and task-owned build frames.
   Build frames own transient DIR state for running and suspended builds.
   Compiler lookups may reuse that transient state only through explicit boundary-aware frame access.
   The immediate targets are:
   - `language/compiler/src/compile/frame.rs`
   - `language/compiler/src/compile/commit.rs`
   - `language/compiler/src/compile/process.rs`
   - `language/compiler/src/compile/compiler.rs`
2. Resolve transient state
   Replace module-owned resolve truth with one transient resolve builder that starts from `DirBase` or `DirPrepared` artifacts and runs inside the current build frame.
   The immediate targets are:
   - `language/compiler/src/resolve/binding/path.rs`
   - `language/compiler/src/resolve/module/prepare.rs`
   - `language/compiler/src/resolve/module/module.rs`
   - `language/compiler/src/resolve/module/canonical.rs`
3. Analyze transient state
   Replace infer and analyze entrypoints that still start from `module.dir(profile)` with builders that start from `DirDeclared`, `DirInterface`, or `DirAnalyzed` artifact snapshots.
   The immediate targets are:
   - `language/compiler/src/analyze/declare/process.rs`
   - `language/compiler/src/analyze/infer/process.rs`
   - `language/compiler/src/analyze/solve/process.rs`
   - `language/compiler/src/analyze/commit/pipeline.rs`
   - `language/compiler/src/analyze/capture/process.rs`
   - `language/compiler/src/analyze/validate/process.rs`
4. Builtin environment builders
   Make environment builders return immutable products directly and remove builtin-side coordination that exists only to support module-local mutable workspace.
   The immediate targets are:
   - `language/compiler/src/resolve/language/builtin.rs`
   - `language/compiler/src/resolve/language/lib.rs`
   - `language/compiler/src/analyze/environment/intrinsic.rs`
   - `language/workspace/src/session/builtin.rs`
5. Lower and elaborate ingress
   Switch elaborate, execute, and lower to artifact-backed transient inputs instead of `module.dir(profile)` truth.
   The immediate targets are:
   - `language/compiler/src/elaborate/`
   - `language/compiler/src/execute/`
   - `language/compiler/src/lower/`
6. Final compiler consumer sweep
   Remove the last compiler reads that still consult module-backed DIR or MIR surfaces and route them through artifact snapshots instead.
   The immediate targets are:
   - `language/compiler/src/cache/signature.rs`
   - `language/compiler/src/generate/cranelift.rs`
   - `language/compiler/src/optimize/pipeline/workset.rs`
   - `language/compiler/src/resolve/module/prepare.rs`
7. Workspace consumer sweep
   Move non-compiler consumers off `module.dir_*` and `module.mir_*` so module-owned derived state can be deleted entirely.
   The immediate targets are:
   - `language/query/`
   - `language/linter/`
   - `language/service/`
   - `language/lsp/`
   - `app/cli/`
   - `service/daemon/`

Exit gates:

- no module-workspace rehydration or mirror bridges remain in compiler or workspace
- no compiler code treats module-local mutable DIR as semantic truth
- no compiler code reads module-backed MIR as semantic truth
- current mutable DIR state is owned by the current task/build frame only
- producers do not start from `module.dir(profile)` as semantic truth
- environment builders commit immutable products directly
- non-compiler consumers read derived semantic state from artifacts, not module-backed `dirs` / `mirs`
- compiler unit and spec baselines remain green

Current slice-3 bridge:

- rebuild one transient mutable `ModuleDir` builder from one committed artifact snapshot at producer ingress

This bridge is acceptable only because it is centralized in `language/compiler/src/compile/frame.rs`.
Slice 4 should replace it with more granular immutable payloads and narrower transient builders.

Current post-slice-4 state:

- module-owned derived `dirs` are deleted
- module-owned derived `mirs` are deleted
- compiler, query, linter, service, daemon, CLI, and codegen consumers now read derived DIR and MIR state from artifacts
- `language/compiler/src/compile/workspace.rs` is gone
- `language/compiler/src/compile/frame.rs` is gone
- `CURRENT_BUILD_FRAME` is gone
- `BuildProduct` is gone
- signature-specific invalidation is gone
- exact artifact requirements now drive availability and staleness
- the semantic compiler path no longer relies on ambiguous `best_available_*` or `dir_snapshot` style reads
- the analyze bridge has been pruned further by deleting `ModuleContext` and `ModuleTreeView`

### Slice 4: finish the core semantic truth pivot

Status:

- complete on the semantic compiler path

What slice 4 deleted:

- `CURRENT_BUILD_FRAME`
- `BuildProduct`
- `ModuleSignature`
- `ModuleSignatureDigest`
- signature-specific invalidation logic
- ambiguity-critical `best_available_*` helper use on the semantic compiler path
- `ModuleContext`
- `ModuleTreeView`

What slice 4 established:

- no second dependency system beside exact artifact requirements
- no mutable module-owned semantic truth container
- no compatibility preservation of signatures or build-frame state
- no consumer-side semantic repair for committed publication
- exact artifact reads across module boundaries

What slice 4 deliberately left for later:

- whether the remaining persistence-only workspace index state can shrink further
- whether any new compiler-local shared cache is actually necessary or should instead be deleted or artifactized
- whether `ModuleDir` can stop being the broad published semantic truth container
- whether `Shared<T>` should be replaced, if and only if that becomes a real simplification instead of mechanical churn

These are intentionally slice-5 items now, not unfinished slice-4 surprises:

- they are real remaining non-final shapes
- but they require broader ownership or API cuts rather than one more local bridge deletion
- forcing them into slice 4 would risk another half-transitional cleanup instead of a clean architectural pass

### Slice 5: final essence pass

Goals:

- reduce the workspace and compiler shape to the smallest architecture that still expresses the real system
- keep only input identity, published artifact truth, exact keys, exact requirements, minimal scheduler state, and cache hydrate or persist boundaries
- remove any remaining noun or verb that does not clearly pay for itself against that minimal architecture
- route persistence exclusively through `ArtifactStore`
- remove ad hoc cache reads and writes from producers
- update linter, query, service, daemon, runtime, and tests to use artifact ingress only
- remove ambient `*_maybe` control flow reads from inner logic
- finish deleting obsolete scheduler and phase vocabulary
- remove duplicate cache conventions and direct cache path logic from compiler internals
- keep deleting support-surface nouns and helper layers that no longer buy real architectural clarity
- challenge every remaining broad semantic container and convenience view against narrower artifact truth
- prefer fewer nouns, fewer verbs, and fewer adapter layers over local ergonomic abstraction

Minimal surviving essence:

- input identity and ingress state in source and workspace registries
- raw source, config, and manifest files in `FileRegistry`
- all derived artifacts in `ArtifactRegistry`
- exact `ArtifactKey`
- exact `ArtifactRequirement`
- minimal scheduler state to realize artifact keys
- cache hydrate and persist around those keys

Correct places to park state:

- raw external file facts in `FileRegistry`
- input identity and mutable ingress stamps in the input registries
- immutable published derived truth in `ArtifactRegistry`
- in-flight execution bookkeeping in the scheduler only
- persisted snapshot state in cache load and store code only

Incorrect places to park state:

- compiler-private caches on `Program`
- query indexes on `Program`
- emitted outputs in `FileRegistry`
- retained serialized cache blobs as live semantic state
- broad mutable working-set nouns that survive phase boundaries
- rebuildable derived state with no exact key and no clear owner

Rules:

- no compatibility bridges
- no old task-based semantic helpers left behind
- no new cache owners introduced just to move state around
- if rebuildable shared state earns its keep, promote it to a real keyed artifact rather than parking it on `Program` or `Compiler`
- if rebuildable shared state does not earn artifact status, keep it local and ephemeral
- if a noun is not input identity, artifact truth, exact key, exact requirement, scheduler state, or cache persistence, it is guilty until proven innocent
- if a verb is not read, publish, invalidate, hydrate, persist, or realize, it is guilty until proven innocent
  
Refined final rule:

- input facts are not artifacts
- everything derived is an artifact
- the scheduler realizes artifact requirements only

Primary deletions:

- ad hoc phase-specific cache IO
- duplicate persistence paths outside `ArtifactStore`
- the separate `OutputRegistry`, `OutputKey`, `OutputStore`, `BuildKey`, and `BuildRequirement` architecture
- legacy query, linter, or service logic that rediscovers semantic truth instead of requiring artifacts
- low-value context and view wrappers that only bundle parameters without expressing a real boundary
- broad published semantic containers that should collapse into narrower artifact families
- helper-matrix APIs whose only job is forwarding local vs remote reads through one more noun
- broad artifact-registry write and delete method forests that exist only because publication and invalidation are still too bespoke
- broad mutable working-set nouns that still mirror published artifact structure
- mutable module-state buckets that still sit next to otherwise immutable input identity
- cache or snapshot state that still sticks to `Program` instead of staying at hydration boundaries
- query-service acceleration state parked on `Program`
- compiler-private rebuildable caches that are not real artifacts
- ambiguous environment or artifact fallbacks that still pick "any available" truth instead of the exact profile or key
- operational scaffolding nouns that are only reporting decoration rather than required execution state

Additional slice-5 pressure tests:

- whether `ModuleDir` can stop being the broad published semantic truth container by splitting the remaining DIR surface into narrower artifact families
- whether the remaining workspace-index payload should keep both file and module version seeds in memory or collapse further
- whether `ArtifactRegistry` has reached the final exact-key publication and invalidation surface or still carries avoidable convenience APIs
- whether any remaining exact-artifact ambiguity helpers like the old `mir_snapshot(...)` shape reappear anywhere
- whether `WorkspaceIndexSnapshot` should survive only as a persistence payload rather than cache-state plumbing exposed through live APIs
- whether the remaining workspace-index file and module version seeds can shrink further without losing useful cold-start wins
- whether any query acceleration should survive as parked state at all now that `ProgramQueryIndex` is gone
- whether the remaining analyze view lattice in `language/compiler/src/analyze/common/phase.rs` still encodes real boundaries or should collapse further
- whether `TaskPhase` is the only scheduler-label noun we really need and whether even more scheduler reporting metadata can collapse into `ArtifactKey` helpers
- whether `require_*` and similar helper verbs still reflect real dependency boundaries rather than historical phase narration
- whether any remaining non-truth derived state still reachable through `Session` or `Compiler` can be removed or promoted to exact artifacts

## Original Ordered Deletion Checklist

This is the original strict removal order for the refactor.
The current completion status is recorded above in `Completion Status For The Deletion Checklist`.

### First: names and ownership

- [ ] Rename generated `Artifact*` output types to `Output*`
- [ ] Remove generated-output `ArtifactRegistry`
- [ ] Introduce `OutputStore`
- [ ] Introduce semantic `ArtifactRegistry` and `ArtifactStore`
- [ ] Remove any mixed semantic and output naming

### Second: old scheduler semantics

- [ ] Remove semantic use of `TaskDependency`
- [ ] Remove `TaskDependencyError`
- [ ] Remove yielded dependency trees over tasks
- [ ] Remove scheduler logic that scans task status for semantic satisfaction
- [ ] Remove scheduler fallback error logic from dependency trees
- [ ] Remove `do_require_task_internal_only`

### Third: old phase vocabulary

- [ ] Remove phase narration `require_*` APIs
- [ ] Remove `AnalyzeDependencyStage`
- [ ] Remove standalone validate boundary
- [ ] Remove solve, commit, capture task facades

### Fourth: module-owned derived state

- [ ] Remove module-owned current DIR slots
- [ ] Remove module-owned infer handoff truth
- [ ] Remove module-owned analyzed, elaborated, comptime, and MIR state
- [x] Remove `ModuleDir` as published semantic truth

### Fifth: second dependency system

- [x] Remove `ModuleSignature`
- [x] Remove `ModuleSignatureDigest`
- [x] Remove signature-specific invalidation logic
- [x] Remove `ProgramIndex` as the general home for derived semantic truth

These two stages should be treated as one continuous implementation arc.
Do not stabilize a half-new producer model by keeping signatures, `ProgramIndex`, or mutable `ModuleDir` alive longer than necessary.

### Sixth: old cache and invalidation logic

- [ ] Remove broad downstream invalidation as a correctness mechanism
- [ ] Remove ad hoc phase-specific cache reads and writes
- [ ] Remove duplicate persistence paths outside `ArtifactStore`
- [ ] Remove ambient inner-logic `*_maybe` artifact lookups as control flow

### Seventh: fallout cleanup

- [ ] Remove legacy linter, query, daemon, runtime, and service code that re-discovers semantic truth
- [ ] Remove any compatibility aliases or reexports created during the rewrite

## Tracked Follow-up Debt

This section exists so temporary bridges and adjacent architectural debt do not disappear into code comments.
Items are grouped by the workstream that should own them.

### Slice 3: ownership cut follow-up

- [x] Replace whole-`ModuleDir` transient rebuilding with immutable shared payloads and direct phase locals
  Producer phases now mutate direct locals and publish exact `Dir*` payloads instead of reviving a generic `ModuleDir` builder.
- [ ] Tighten resolve access layering in [language/compiler/src/resolve/binding/path.rs](language/compiler/src/resolve/binding/path.rs)
  Slice 3 removed module workspace fallback and remote shared-frame reads.
  The remaining explicit prelude and remote snapshot threading should collapse into one cleaner resolve access layer.
- [ ] Narrow task-local build-frame access in [language/compiler/src/compile/frame.rs](language/compiler/src/compile/frame.rs)
  Build-frame ownership is now the correct model for in-flight mutable DIR.
  Slice 4 should reduce the remaining generic frame helpers to builder-specific surfaces where that actually buys clarity.
- [ ] Rename the old cache vocabulary from `DirExecuted` to `DirPatched`
  Slice 3 fixed semantic ownership and artifact naming, but cache-layer naming still uses the pre-artifact `DirExecuted` term in [language/compiler/src/cache](language/compiler/src/cache) and [language/workspace/src/program/cache.rs](language/workspace/src/program/cache.rs).
  That rename is worthwhile, but it is not an ownership blocker for this slice.
- [ ] Carry stable relation-surface type ids through diagnostics directly
  Slice 3 currently recovers the expected declared type surface from the diagnostic node in [language/compiler/src/analyze/common/type.rs](language/compiler/src/analyze/common/type.rs) when normalized relation types collapse to `unknown` or `unevaluated`.
  Slice 4 should make relation diagnostics carry stable surface ids explicitly instead of recovering them late.

### Post-refactor workstream: analyze and inference engine

- [ ] Fix missing substitution and instantiation context in analyze caches
  Tracked in:
  [language/compiler/src/analyze/static/substitute.rs](language/compiler/src/analyze/static/substitute.rs),
  [language/compiler/src/analyze/declare/type/cache.rs](language/compiler/src/analyze/declare/type/cache.rs),
  [language/compiler/src/analyze/infer/provisional/resolution.rs](language/compiler/src/analyze/infer/provisional/resolution.rs)
- [ ] Revisit analyze validation pass structure
  Validation is correctly folded into `DirAnalyzed`, but [language/compiler/src/analyze/validate/process.rs](language/compiler/src/analyze/validate/process.rs) still notes multiple full tree passes.
- [ ] Preserve declared relation surfaces for diagnostics without mutating authored annotation slots
  Tracked by the known-failure spec case `types/template-literals/numeric-spans.md/numeric-spans/template-literal-type-rejects-invalid-bigint-strings`.
  This should be solved in the analyze/inference workstream by carrying stable relation-surface ids instead of reconstructing them after declaration resolution mutates the slot.
- [ ] Remove remaining declaration-time owner/static-parameter quirks
  Tracked in [language/compiler/src/analyze/declare/collect/declaration.rs](language/compiler/src/analyze/declare/collect/declaration.rs)
- [ ] Revisit dynamic operator and union-resolution gaps
  Tracked by ignored/incomplete elaborate tests and operator notes under `language/compiler/src/analyze/` and `language/compiler/src/elaborate/tests/`
- [ ] Move temporary analyze warnings to the lint pipeline once the post-refactor lint ingress settles
  Tracked in [language/compiler/src/analyze/infer/operator/try.rs](language/compiler/src/analyze/infer/operator/try.rs)

### Broader compiler and workspace cleanup

- [ ] Decide whether `Compiler` should stay cross-target or become target-scoped
  Tracked in [language/compiler/src/compile/compiler.rs](language/compiler/src/compile/compiler.rs)
- [ ] Fix VM managed aggregate execution at the representation boundary
  Tracked in [language/vm/src/interpreter/execute/instruction.rs](language/vm/src/interpreter/execute/instruction.rs),
  [language/vm/src/interpreter/dispatch/aggregate.rs](language/vm/src/interpreter/dispatch/aggregate.rs),
  and [language/vm/src/interpreter/decode/threading.rs](language/vm/src/interpreter/decode/threading.rs).
  This is the current blocker to full-system green, but it is not a slice-5 compiler or workspace architecture task.
- [ ] Revisit fallback profiles in `Program`
  Tracked in [language/workspace/src/program/program.rs](language/workspace/src/program/program.rs)
- [x] Remove the remaining `Program` handle to `WorkspaceIndexState`
  `Program` no longer carries workspace-index state.
- [x] Remove `WorkspaceIndexState` as a separate live noun
  Loaded workspace-index version seeds now live directly on `Session`, and `WorkspaceIndexSnapshot` remains only the persistence payload.
- [x] Collapse `ModuleRegistry` input metadata into one owned entry shape
  Mutable input metadata is no longer spread across parallel registry maps.
- [x] Remove query acceleration state from `Program`
  `ProgramQueryIndex` is gone and no replacement query cache is parked on `Program`.
- [x] Remove old telemetry or bookkeeping nouns that were only decoration
  `TaskSkipReason` and other stale reporting-only nouns are gone.
  `TaskPhase` remains as derived execution vocabulary over artifact keys, which is an acceptable final scheduler label.
- [x] Remove remaining target-selection helper nouns that were only historical cache vocabulary
  `GlobalSymbolTableKey` is gone.
- [ ] Revisit comptime execution performance and target-awareness
  Tracked in [language/compiler/src/execute/comptime.rs](language/compiler/src/execute/comptime.rs)
- [ ] Revisit MIR lowering ownership and pointer-width placement
  Tracked in [language/compiler/src/lower/process.rs](language/compiler/src/lower/process.rs)
- [ ] Finish RTTI and interface-upcast support in lowering
  Tracked in [language/compiler/src/lower/emit/value/cast.rs](language/compiler/src/lower/emit/value/cast.rs) and [language/compiler/src/lower/table/rtti.rs](language/compiler/src/lower/table/rtti.rs)
- [ ] Revisit remaining performance hotspots called out in solve, infer, assignability, validation, and lower
  These are not blockers for the state-model refactor, but they are now explicitly tracked rather than left as scattered notes.
- [ ] Keep slimming `Dir*` payload contracts in later resolve and analyze refactors
  The generic `ModuleDir` bag is gone, but some exact `Dir*` payloads still carry more copied-forward surface than their real downstream contracts need.

## Deletion Checklist

- [ ] Rename generated `Artifact*` output types to `Output*`
- [ ] Remove semantic use of `TaskDependency`
- [ ] Remove `TaskDependencyError`
- [ ] Remove scheduler logic that scans task status for semantic satisfaction
- [ ] Remove `do_require_task_internal_only`
- [ ] Remove phase narration `require_*` APIs
- [ ] Remove `AnalyzeDependencyStage`
- [ ] Remove standalone validate boundary
- [ ] Remove solve, commit, capture task facades
- [ ] Remove `ModuleSignature`
- [ ] Remove `ModuleSignatureDigest`
- [ ] Remove signature specific invalidation logic
- [ ] Remove broad downstream invalidation as a correctness mechanism
- [ ] Remove ambient inner logic `*_maybe` artifact lookups as control flow
- [ ] Remove module-owned mutable derived phase state
- [x] Remove `ModuleDir` as published semantic truth
- [ ] Remove any compatibility aliases or reexports created during the rewrite
- [ ] Remove module input version updates caused by semantic downstream changes
- [ ] Remove scheduler or cache logic that treats rerun as semantic change without digest change
- [ ] Remove transitive semantic dependency blobs where direct artifact dependencies suffice

## Locked Decisions

- `Task` stays as an execution noun
- artifacts are the only semantic truth
- the scheduler schedules artifact builders only
- validation is part of `DirAnalyzed`
- interface is published per module, with SCC evaluation internal only
- elaborate and comptime remain distinct artifact boundaries
- caches are registry hydration and persistence only
- emitted outputs are artifacts
- modules own input state only
- input versions remain input-only
- digest propagation is red-green
- diagnostics are tied to build attempts but are not artifact validity
