# Destack Daemon

Destack background daemon.
Persistent service for watch mode, caching, and incremental compilation.

## Responsibilities

The daemon owns the long lived workspace session and all incremental state.
Responsibilities include:

- file watching and change coalescing
- incremental compilation and diagnostics
- cache management for in memory and on disk caches
- REPL and notebook execution via the VM
- hot reload orchestration for supported runtimes
- LSP and CLI request coordination

## Services

### Compile Service

The compile service applies file changes, bumps versions, and schedules tasks.
It returns diagnostics, artifacts, and updated module signatures.

### Watch Service

The watch service drives the file watcher and batches edits for compile.
The watcher implementation is pluggable and can be replaced in tests.

### REPL Service

The REPL service maps each cell to a synthetic module and executes it in a persistent VM isolate.
It replays affected cells after edits using the same incremental compilation pipeline.

### Cache Service

The cache service exposes in memory and on disk caches keyed by configuration and compiler version.
Cache policy is configured in dsconfig and enforced by the daemon.
The daemon uses the canonical cache format and writes sidecar metadata for service-specific needs.

### LSP Bridge

The LSP bridge translates editor changes into overlay updates and compile requests.
It never owns compiler state and always delegates to the daemon session.
