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