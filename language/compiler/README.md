# Compiler

The main job of the compiler is to turn _sources_ into final _outputs_ for various _targets_.
The compiler (mostly) operates over our three primary IRs - AST, DIR, MIR - and most of the actually interesting stuff happens over DIR (resolution, analysis, elaboration) and MIR (lowering, optimization, codegen).

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

## Profiles and Targets

A profile is a "comptime world", a set of libraries and builtins and restrictions.
Anything that can affect the compile-time resolution, analysis and execution results is keyed by `Profile`.
Targets, on the other hand, define final output products like a bundled `.js` file, a package of `.js`/`.d.ts` files, a single `.wasm` library, or a final `.exe` executable.

Multiple targets can share the same profile if they're based on the same "comptime world", like if they target the same platform and builtins.
In general, the front-end operates per-Profile, and once we get closer to actual output generation (like for Lower) we split products per-Target.

## Scheduling and Caching

The compiler schedules work over `ArtifactKey`s and records the exact source and artifact inputs used to produce each artifact.
We store those `Artifact`s in-memory in an `ArtifactStore` and on disk via `ArtifactCache`.
For `Dir*`, the on-disk image also validates the restored workspace string universe from `WorkspaceIndex`, because DIR still uses shared `StringId` identity.
On the way, the scheduler deduplicates in-flight work by artifact key and discards stale completed work when dependencies have drifted.
That's pretty much it.

There are many ways of orchestrating a compiler-like program, and one robust way of maintaining incrementally updatable state is Salsa-style querying (with caching per-query) like `rustc` and `ty` use.
That is a good approach, but we wanted to optimize for straightline big-chunk throughput and didn't want to delegate state layout and scheduling to another library.
Whether reifying artifacts and dependencies in this way was such a great idea is up for the reader to judge.

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
