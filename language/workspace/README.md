# workspace

The workspace crate owns most of the shared Destack language toolchain state.
We don't define (much) logic here, it's mostly about defining the containers and registries that make compiler, daemon, tests, and language tools work.

## State

The workspace crate exists to make one ownership split explicit:

- input state lives on long-lived containers such as `Workspace`, `Program`, `Package`, and `Module`
- derived semantic compiler products live in `ArtifactRegistry`
- generated and linked outputs live in `OutputRegistry`

Scheduling should still see one unified build graph.
The right execution key space is:

- `BuildKey::Artifact(ArtifactKey)`
- `BuildKey::Output(OutputKey)`

This distinction is the backbone of the incremental model.

`Module` owns input state only.
That includes things like identity, source linkage, source metadata, and versioned input-side facts.
It does not own current DIR, analyzed state, comptime state, MIR state, or any other derived semantic phase state.

`Program` is the semantic world.
It owns the registries, profiles, diagnostics, and the set of packages and modules that participate in one coherent compilation world.

## Configuration

The `config/` module defines unified toolchain configuration.
That includes compiler, runtime, formatter, linter, cache, daemon, and target options.

### dsconfig.json

Destack's project configuration is similar to `tsconfig.json`, but `dsconfig.json` covers the full integrated toolchain.
(We do still read and respesct `tsconfig.json` for compatibility where an equivalent Destack option exists.)

```json
{
    "compilerOptions": {
        "target": "es2024",
        "strict": true,
        "noImplicitAny": true,
        "incremental": true
    },
    "runtimeOptions": {
        "execution": "record",
        "replayLog": { "path": ".destack/runtime/replay" },
        "time": { "mode": "virtual" },
        "random": { "mode": "deterministic", "seed": 1337 }
    },
    "cache": {
        "mode": "disk",
        "dir": ".destack/cache",
        "maxSizeMb": 2048,
        "policy": "lru",
        "validate": "strict"
    },
    "watch": {
        "debounceMs": 30
    },
    "daemon": {
        "idleShutdownMs": 600000
    },
    "formatter": {
        "lineWidth": 100,
        "indentStyle": "space"
    },
    "linter": {
        "preset": "recommended"
    },
    "targets": {
        "web": { "output": "js", "platform": "browser" },
        "native": { "output": "wasm", "optimize": true }
    }
}
```

Child packages inherit from parent `dsconfig.json` with restrictive merge semantics where appropriate.

## Containers

The containment model is intentionally simple.

```text
Session
    │
    ├── Workspace
    │       │
    │       └── Program
    │               │
    │               ├── Package
    │               │       └── Module
    │               │
    │               ├── ArtifactRegistry
    │               └── OutputRegistry
```

### Session

`Session` is long-lived process state for daemon, editor, and multi-root workflows.
It owns shared input-side resources such as:

- filesystems
- file registries
- cache stores
- builtin sources
- program lookup and reuse

`Session` is not a semantic owner.
Semantic truth belongs to each `Program`.

### Workspace

`Workspace` is the discovered project and monorepo structure from disk.
It describes roots, config, package relationships, and workspace-level organization.

### Program

`Program` is the semantic world for one compilation context.
It owns packages, modules, profiles, diagnostics, the semantic compiler product registry, and the output registry.

### Package

`Package` is the package-level input container within a program.
It participates in package metadata, module membership, and workspace structure.

### Module

`Module` is the source-level input container.
It owns input identity and input metadata.
It does not own derived semantic phase state.

## Semantic And Output State

### ArtifactRegistry

`ArtifactRegistry` owns published semantic compiler products for one program.
The long-term semantic compiler product vocabulary is:

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

These are immutable snapshots built from shared substructures.
They are compiler facts.

`LanguageEnvironment`, `IntrinsicEnvironment`, and `LibEnvironment` are profile-scoped semantic environments.
They are not DIR artifacts.
They are semantic environments derived from builtin and lib module surfaces.

### OutputRegistry

`OutputRegistry` owns generated and linked outputs.
These are build products rather than semantic compiler facts.

Keeping outputs separate from semantic compiler products avoids conflating compiler truth with generated files.

`OutputKey` identifies the logical output product.
`OutputContent` is the payload stored for that key.

That split matters because outputs can share one scheduling model without pretending they are semantic compiler products.
The compiler should be able to drive:

- output to artifact dependencies
- output to output dependencies

without ever letting artifacts depend on outputs.

### Builtins

`Builtins` should become immutable input-side catalog state.
It should own builtin module ids, builtin lib metadata, and other builtin source identity.

It should not own mutable per-profile derived semantic state like:

- language item caches
- declared lib symbol caches
- ambient lib symbol caches
- well-known symbol caches
- intrinsic caches

Those belong in `ArtifactRegistry` as `LanguageEnvironment(profile)`, `IntrinsicEnvironment(profile)`, and `LibEnvironment(profile)`.

## Incremental State

The incremental model separates input state from derived state.

Input state still uses mutable versions and stamps.
Examples include:

- file versions
- module versions
- package versions
- profile and configuration stamps

Derived semantic compiler products are immutable snapshots.
Each published semantic compiler product stores:

- an `ArtifactDependency`
- an `ArtifactDigest`
- an immutable artifact value

Correctness invalidation is lazy.
An artifact is stale when its stored dependency no longer matches the expected dependency.
The workspace should not need broad eager downstream invalidation walks for correctness.

## Caching And Persistence

The workspace crate owns the cache root and the persistence layering used by compiler, daemon, and LSP.

The intended stack is:

1. `FileSystem` for source files and user-visible emitted files
2. `CacheStore` for shared cached byte persistence
3. `ArtifactStore` for typed semantic compiler product persistence
4. `OutputStore` for typed output persistence
5. `ArtifactRegistry` and `OutputRegistry` for authoritative in-memory state

The in-memory registries are authoritative.
Persistent cache is hydration and persistence for published semantic compiler products and outputs.

The scheduler should operate over `BuildKey`, not directly over cache paths or phase tasks.
That keeps persistence, execution, and semantic truth properly separated.

The cache flow is:

1. check the in-memory registry
2. try to hydrate the requested artifact or output
3. if still missing, build it
4. on successful commit, persist it

This keeps file IO, low-level cached bytes, typed cached state, and in-memory truth cleanly separated.

## `.destack`

The workspace crate participates in the repository-wide state layout:

- `~/.destack` for install-level shared resources
- `repo/.destack` for workspace-local mutable state

Workspace-owned language state should live under:

- `repo/.destack/language/workspace`

That root is the natural home for workspace-owned cached persistence.
It keeps the path layout aligned with semantic ownership rather than splitting cache paths by artifact category first.

## File Watching

File watching is abstracted by `FileWatcher` in `destack_source`.
The workspace consumes watcher events and updates input versions and input-side graph state.

## Targets

Targets define build outputs and runtime placement.
Runtime and platform define semantics and APIs.
Target triples define native ABI and architecture.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_workspace
just language/test-query
just language/test-lsp

# clean gate
just language/quick

# exhaustive gate
just language/full
```
