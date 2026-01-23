# Destack Daemon

Destack background daemon service.
The daemon is a per workspace, long lived coordinator for toolchain services.
It owns incremental compiler state today, and is the integration point for runtime, REPL, notebook, and hot reload services as they land.

## Responsibilities

The daemon owns the long lived workspace session and all incremental state.
Responsibilities include:

- file watching and change coalescing
- incremental compilation and diagnostics
- cache management for in memory and on disk caches
- LSP and CLI request coordination
- REPL and notebook execution via the VM (planned)
- hot reload orchestration for supported runtimes (planned)
- runtime and library platform services (planned)

## Lifecycle

The daemon is auto spawned by CLI and LSP clients when needed.
Each workspace root maps to its own daemon instance and socket.
Clients connect by discovering the instance metadata and performing a protocol handshake.
When no client connections or workspace handles remain, the daemon will idle shut down after the configured timeout.
Use `daemon.idleShutdownMs` in `dsconfig.json` to tune or disable auto shutdown (set to `0` to disable).

## Commands

Use CLI commands to manage daemon instances:

- `destack daemon serve` runs a foreground daemon server
- `destack daemon start` ensures a daemon is running
- `destack daemon status` reports daemon connectivity
- `destack daemon stop` requests shutdown
