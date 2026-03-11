# Compiler

The compiler turns source modules into semantic compiler products, then into build products.
Its job is not just to "run phases".
Its job is to maintain a coherent semantic world, publish reusable derived facts, and realize requested results efficiently.

The design target is an artifact-first incremental compiler.
That means:

- `Program` is the semantic world
- semantic truth is stored as published semantic compiler products
- dependencies are artifact to artifact
- tasks are execution only
- generated files and binaries are build products, not semantic compiler products

Most of the compilation logic lives in `language/compiler`, with a few externalized crates for neighboring concerns:

- `language/parser` parses code modules
- `language/resolver` resolves module paths
- `language/codegen` owns most target code generation
- `language/linter` owns lint logic

## Overview

The compiler operates over three representation families.

| Representation | Role |
| --- | --- |
| AST | Syntax tree close to source text |
| DIR | Semantic IR with scopes, symbols, types, exports, and language-level structure |
| MIR | Monomorphic, target-aware IR for comptime, optimization, and backend work |

The stable semantic compiler product vocabulary is:

- `LanguageEnvironment`
- `IntrinsicEnvironment`
- `LibEnvironment`
- `Ast`
- `DirBase`
- `DirPrepared`
- `DirResolved`
- `DirDeclared`
- `DirInterface`
- `DirAnalyzed`
- `DirElaborated`
- `DirPatched`
- `Mir`
- `MirOptimized`

Those are semantic products.
Generated JS, source maps, wasm, objects, bundles, and linked binaries are outputs and belong to `OutputRegistry`.

The scheduler should not distinguish these by phase tasks.
It should drive one unified build graph over:

- `BuildKey::Artifact(ArtifactKey)`
- `BuildKey::Output(OutputKey)`

## Pipeline

The compiler still has a familiar phase structure, but the phase names are not the semantic dependency model.
They are just the most natural way to describe how artifact producers are organized.

The front-end turns source into progressively richer DIR products.
The middle-end executes comptime code and lowers DIR to MIR.
The back-end turns MIR products into outputs.

## Ownership

The compiler assumes a strict ownership split.

- `Program` owns the semantic world
- `ArtifactRegistry` owns all published semantic compiler products for that world
- `OutputRegistry` owns generated and linked outputs
- `Module` owns input state only
- transient builders own mutable phase-local work during execution

This is the key long-term rule.
Modules do not own current DIR, inference state, analyzed state, elaborated state, patched state, or MIR state.
If a piece of state is derived semantic truth, it belongs in the artifact registry.

## Artifact Model

Each artifact family has:

- a semantic identity, expressed by `ArtifactKey`
- a validity contract, expressed by `ArtifactDependency`
- a stable semantic summary, expressed by `ArtifactDigest`
- one producer recipe

Published artifacts are immutable snapshots.
They are built from shared immutable substructures rather than deep-cloned whole-module state.

The intended DIR decomposition is coarse-grained and shared:

- core module metadata and roots
- node tree data
- symbol and scope data
- type and resolution data
- capture data
- export and interface data

Different artifact families do not need identical payload shapes.
`DirInterface` in particular should be a compact published interface view, not a full cloned analyzed DIR.

The same is true for profile-scoped environments.
`LanguageEnvironment`, `IntrinsicEnvironment`, and `LibEnvironment` are semantic environments derived from builtin and lib module surfaces.
They are not DIR snapshots and should not be forced into the `Dir*` family.

## Tasks

Tasks are execution frames for build keys.
Each task realizes one `BuildKey` against one captured dependency attempt.

`BuildKey` is the scheduler key space.
It spans:

- semantic compiler products in `ArtifactRegistry`
- output products in `OutputRegistry`

The scheduler is deliberately operational and narrow in scope.
It should:

- deduplicate in-flight builds by build key
- schedule missing build requirements
- resume waiters when artifacts or outputs become available
- discard stale completed work when dependencies drift before commit

The scheduler should not:

- encode semantic dependency policy in task terms
- interpret diagnostics as semantic truth
- compensate for producers that mixed diagnostics with publication incorrectly

In other words, semantic code asks for semantic compiler products, output producers ask for semantic compiler products or upstream outputs, and the scheduler only realizes those requests.

## Interface Fixed Point

The interface phase is the only place where the compiler needs fixed-point style evaluation across cyclic module groups.
The published compiler product is still per-module `DirInterface(module, profile)`.

Internally, the compiler may evaluate an SCC together and commit a converged batch atomically.
That batch behavior is an execution detail, not part of the public artifact vocabulary.

This keeps the semantic model simple while still allowing the correct algorithm for cyclic interface surfaces.

## Builtins And Globals

The compiler needs two profile-scoped semantic environments in addition to the module IR chain.

- `LanguageEnvironment(profile)`
- `IntrinsicEnvironment(profile)`
- `LibEnvironment(profile)`

`LanguageEnvironment` is the compiler-known language environment for one profile.
It includes language items, prelude-facing builtin bindings, and similar compiler-known builtin state.

`IntrinsicEnvironment` is the profile-scoped intrinsic binding environment.
It includes well-known intrinsic bindings derived from builtin declaration decorators.

`LibEnvironment` is the selected library environment for one profile.
It includes selected lib modules, declared lib symbols, merge sources, and well-known symbol state.

These should be published semantic compiler products.
They should not live as mutable per-profile caches on `Builtins`.

That means `Builtins` should become an immutable builtin catalog.
It should describe builtin modules and builtin lib metadata as input state.
The semantic environments derived from that catalog belong in `ArtifactRegistry`.

## Incrementality

The incremental model separates input state from derived semantic state.

Input-side versions and stamps still exist for:

- files
- modules
- packages
- profiles and configuration

Those cheap mutable inputs are used to compute expected artifact dependencies.

Published artifacts then store:

- the exact dependency they were built against
- a stable digest of their semantic value

Downstream artifacts depend on upstream digests.
Correctness invalidation is lazy.
An artifact becomes stale when its stored dependency no longer matches the expected dependency.

This is why the compiler does not need correctness-critical downstream invalidation walks.
Eager clearing may still exist for cleanup or memory pressure, but not as the correctness model.

## Outputs

Outputs are build products with their own identity and payload model.

`OutputKey` identifies which logical output product we are talking about.
Examples include generated module outputs, linked package outputs, and emitted destination outputs.

`OutputContent` is the payload for one `OutputKey`.
That payload may be:
- a text file
- a binary blob
- a source map
- a link product
- an emission bookkeeping record

## Caching

The in-memory artifact registry is authoritative.
Disk cache is hydration and persistence for published semantic compiler products.

The persistence stack should stay layered:

- `FileSystem` for source files and user-visible emitted files
- `CacheStore` for shared cached bytes
- `ArtifactStore` for typed semantic compiler product persistence
- `OutputStore` for typed output persistence

The cache flow is:

1. check the in-memory registry
2. try to hydrate the requested artifact or output
3. if still missing, build it
4. on successful commit, persist it

Not every artifact family must be persisted, but every persisted semantic compiler product should go through `ArtifactStore`.

## Profiles, Targets, And Comptime

A profile is a semantic configuration.
It determines which builtins and libraries exist, which restrictions apply, and what comptime-visible environment is in scope.

A target is a concrete build output configuration.
Multiple targets may share the same profile for front-end work.

This split matters because the compiler should reuse front-end artifacts across targets wherever possible.
That is why `Mir` and `MirOptimized` are keyed by `(module, profile, target)`, while most DIR products are keyed only by `(module, profile)`.

## Builtins

Language builtins live in `language/builtin/`.
The compiler loads builtin modules and declaration libraries as needed from that source set.

## Structure

The compiler is organized into modules (roughly) corresponding to each phase, plus some additional administrative modules (like `compile/` and `unbind/` and `tests/`).

| Path | Description | Source |
| --- | --- | --- |
| `compile/` | Task execution, queueing, cache integration, and orchestration | [src/compile/](src/compile/) |
| `import/` | Parse, bind, and desugar into early DIR state | [src/import/](src/import/) |
| `resolve/` | Resolve names and module references | [src/resolve/](src/resolve/) |
| `analyze/` | Declaration, interface, inference, validation, and checking | [src/analyze/](src/analyze/) |
| `elaborate/` | Post-analysis canonicalization and lowering-oriented rewrites | [src/elaborate/](src/elaborate/) |
| `execute/` | Comptime execution and DIR patching | [src/execute/](src/execute/) |
| `lower/` | Lower DIR to MIR | [src/lower/](src/lower/) |
| `optimize/` | Verify and optimize MIR | [src/optimize/](src/optimize/) |
| `generate/` | Produce target outputs from DIR or MIR | [src/generate/](src/generate/) |
| `link/` | Link outputs into final build products | [src/link/](src/link/) |
| `emit/` | Write final outputs to disk | [src/emit/](src/emit/) |
| `unbind/` | DIR to AST utilities | [src/unbind/](src/unbind/) |
| `tests/` | Compiler test scaffolding | [src/tests/](src/tests/) |

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_compiler
cargo test -p destack_test --test smoke -- --compiler
just language/test-specification
just language/test-query

# clean gate
just language/quick

# exhaustive gate
just language/full
```
