# Destack Daemon

Destack background daemon service.
The daemon is a per workspace, long lived coordinator for toolchain services.
It owns incremental compiler state today, and is the integration point for runtime, REPL, notebook, and hot reload services as they land.

The daemon is intentionally local and workspace scoped.
It should remain the coordinator for local language and runtime workflows, not the place where remote deployment, CI, or broader platform orchestration logic accumulates.

## Responsibilities

The daemon owns the long lived workspace session and all incremental state.
Responsibilities include:

- file watching and change coalescing
- incremental compilation and diagnostics
- cache management for in memory and on disk caches
- CLI and automation request coordination
- REPL and notebook execution via the VM (planned)
- hot reload orchestration for supported runtimes (planned)
- runtime and library foundational services (planned)

## Lifecycle

The daemon is auto spawned by clients when needed (e.g., on the CLI).
Each workspace root maps to its own daemon instance and socket.
After a while, when no client connections or workspace handles remain, the daemon will idle shut down after the configured timeout.

## Commands

Use CLI commands to manage daemon instances:

- `destack daemon serve` runs a foreground daemon server
- `destack daemon start` ensures a daemon is running
- `destack daemon status` reports daemon connectivity
- `destack daemon stop` requests shutdown

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_daemon

# clean gate
just service/quick

# exhaustive gate
just service/full
```
