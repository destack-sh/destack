# Destack Incremental Compilation + REPL + Daemon Plan (Working Draft)

This file is a temporary planning aid and should not be committed.

## Purpose

This plan captures the decisions and next steps for incremental compilation, caching, REPL, daemon, and LSP integration.

## Guiding principles

These are the constraints we agreed to keep.

- Keep the compiler pipeline straight line and predictable, not salsa style.
- Use ModuleId, ModuleVersion, ProfileVersion, and FileVersion as the coarse invalidation units.
- Align with JS and TS behavior where possible, while extending where Destack adds value.
- Treat cache correctness as strict, never reuse invalid or incompatible data.
- Prefer explicit plumbing over hidden magic, especially for incremental and caching.

## Current status (implemented)

These items are already landed in the repo.

- File watcher abstraction lives in `language/source` alongside the filesystem.
- Cache registry supports memory and disk, with strict validation and locking.
- Cache stats are surfaced in CLI JSON output and text summaries for check and build.
- Config hashing includes resolved target configuration plus dsconfig base fields.
- AST, DIR, and MIR serialization with cache headers is implemented.

## Decisions (agreed or leaning)

These summarize the choices we already discussed.

- Incremental compilation uses module-level invalidation, not fine-grained query caches.
- Cache invalidation is strict: version mismatch is a miss, not an error.
- Cache directory defaults to `.destack/` but accepts TS style aliases where sensible.
- Dynamic evaluation is a runtime feature, not a comptime feature.
- Daemon owns file watching and long-lived compiler state; CLI uses it when requested.
- REPL should mirror JS/TS semantics where possible, but use Destack’s compiler pipeline.

## Open questions (to decide before incremental core)

These decisions affect cache keys, invalidation, and REPL semantics.

- Module signature granularity: per export, per module, or hybrid.
- REPL cell model: append to a synthetic module vs one module per cell.
- Dynamic eval API shape: reuse `eval`/`Function`, or add a Destack-native API.
- Serialization format: stick with bincode or move to another binary format later.
- Daemon boundaries: which APIs live in `language/workspace` vs `platform/daemon`.

## Workstreams and order

This is the recommended execution order.

1) Incremental compilation core.
2) Cache integration in the pipeline.
3) LSP and watch mode incremental tests.
4) REPL and daemon integration.

## Workstream 1: incremental core

These are the concrete tasks to make incremental compilation real.

- Define module signature structure and how it is computed.
- Define invalidation rules from file changes and signature changes.
- Hook module and task dependency tracking into the compiler task graph.
- Define cache keys for module, profile, target, compiler version, and dsconfig.
- Add tests using MemoryFileSystem and FileWatcher for incremental behavior.

## Workstream 2: cache integration

These are the tasks to connect cache reads and writes to the compiler.

- Load cached AST, DIR, and MIR when cache keys match and modules are valid.
- Write cache entries at stable boundaries in the pipeline.
- Ensure cache reads never block compilation, and fall back cleanly to recompute.
- Add targeted tests for serialization roundtrips and cache invalidation.

## Workstream 3: LSP and watch mode tests

These are the tasks to validate the incremental system in practice.

- Define a FileWatcher test harness alongside MemoryFileSystem.
- Simulate edit sequences and verify diagnostics match expectations.
- Add end to end tests that validate cache hits and invalidations.

## Workstream 4: REPL and daemon

These are the tasks to ship a first class REPL and daemon.

- Define REPL compilation units and state retention rules.
- Add daemon interfaces for watch, incremental compile, and REPL sessions.
- Implement CLI to daemon wiring for `--watch` and `repl`.
- Add runtime hooks for dynamic evaluation and module injection.

## Testing strategy

These are the minimum tests we should add as we go.

- Cache read and write roundtrips for AST, DIR, and MIR.
- Incremental invalidation tests for file edits and module signature changes.
- FileWatcher tests for add, change, remove, and rename.
- LSP tests that verify diagnostics stability across edits.

## Documentation plan

These updates should be made once decisions are final.

- Update `language/compiler/README.md` with incremental rules and cache keys.
- Update `language/workspace/README.md` with daemon and watch behavior.
- Update `language/DESIGN.md` with REPL semantics and dynamic eval rules.
