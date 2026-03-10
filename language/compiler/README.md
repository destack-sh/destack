# Compiler

The compiler turns source modules into semantic artifacts, then into build outputs.
Its job is not just to "run phases".
Its job is to maintain a coherent semantic world, publish reusable derived facts, and realize requested results efficiently.

The design target is an artifact-first incremental compiler.
That means:

- `Program` is the semantic world
- semantic truth is stored as published artifacts
- dependencies are artifact to artifact
- tasks are execution only
- generated files and binaries are outputs, not semantic artifacts

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

The stable semantic artifact vocabulary is:

- `Ast`
- `DirBase`
- `DirResolved`
- `DirDeclared`
- `DirInterface`
- `DirAnalyzed`
- `DirElaborated`
- `DirComptime`
- `Mir`

Those are semantic products.
Generated JS, source maps, wasm, objects, bundles, and linked binaries are outputs and belong to `OutputRegistry`.

## Pipeline

The compiler still has a familiar phase structure, but the phase names are not the semantic dependency model.
They are just the most natural way to describe how artifact producers are organized.

The front-end turns source into progressively richer DIR artifacts.
The middle-end executes comptime code and lowers DIR to MIR.
The back-end turns semantic artifacts into outputs.

```text
Ast
  │
  ▼
DirBase
  │
  ▼
DirResolved
  │
  ▼
DirDeclared
  │
  ├──► DirInterface
  ▼
DirAnalyzed
  │
  ▼
DirElaborated
  │
  ▼
DirComptime
  │
  ▼
Mir
  │
  ▼
outputs
```

This sequence is the semantic backbone of the compiler.
Tasks and scheduling should reflect it.
They should not replace it.

## Ownership

The compiler assumes a strict ownership split.

- `Program` owns the semantic world
- `ArtifactRegistry` owns all published semantic artifacts for that world
- `OutputRegistry` owns generated and linked outputs
- `Module` owns input state only
- transient builders own mutable phase-local work during execution

This is the key long-term rule.
Modules do not own current DIR, inference state, analyzed state, elaborated state, comptime state, or MIR state.
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

## Tasks

Tasks are execution frames for artifact builds.
Each task builds one artifact key against one captured dependency attempt.

The scheduler is deliberately operational and narrow in scope.
It should:

- deduplicate in-flight builds by artifact key
- schedule missing artifact requirements
- resume waiters when artifacts become available
- discard stale completed work when dependencies drift before commit

The scheduler should not:

- encode semantic dependency policy in task terms
- interpret diagnostics as semantic truth
- compensate for producers that mixed diagnostics with publication incorrectly

In other words, semantic code asks for artifacts.
The scheduler only realizes those requests.

## Interface Fixed Point

The interface phase is the only place where the compiler needs fixed-point style evaluation across cyclic module groups.
The published artifact is still per-module `DirInterface(module, profile)`.

Internally, the compiler may evaluate an SCC together and commit a converged batch atomically.
That batch behavior is an execution detail, not part of the public artifact vocabulary.

This keeps the semantic model simple while still allowing the correct algorithm for cyclic interface surfaces.

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

## Caching

The in-memory artifact registry is authoritative.
Disk cache is hydration and persistence for published artifacts.

The persistence stack should stay layered:

- `FileSystem` for source files and user-visible emitted files
- `CacheStore` for shared cached bytes
- `ArtifactStore` for typed semantic artifact persistence
- `OutputStore` for typed output persistence

The cache flow is:

1. check the in-memory registry
2. try to hydrate the requested artifact or output
3. if still missing, build it
4. on successful commit, persist it

Not every artifact family must be persisted, but every persisted semantic artifact should go through `ArtifactStore`.

## Profiles, Targets, And Comptime

A profile is a semantic configuration.
It determines which builtins and libraries exist, which restrictions apply, and what comptime-visible environment is in scope.

A target is a concrete build output configuration.
Multiple targets may share the same profile for front-end work.

This split matters because the compiler should reuse front-end artifacts across targets wherever possible.
That is why `Mir` is keyed by `(module, profile, target)`, while most DIR artifacts are keyed only by `(module, profile)`.

The compiler also maintains a shared comptime target for execute.
That target is operationally important even though it is not itself one of the final emitted targets.

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
| `lint/` | Lint rules and diagnostics | [src/lint/](src/lint/) |
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
