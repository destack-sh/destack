# Runtime

The runtime is how Destack actually does anything meaningful beyond pure computation.
It wraps VM and/or native execution with scheduling, bindings, record/replay, and runtime rules.
In effect, the runtime is where we marry Node/Bun/Deno-level semantics _with_ V8/JSC-runtime features.

## Overview

Destack has a single runtime that can drive both VM and native execution (even within the same process).
The runtime owns everything outside of pure computation (and userland external bindings): time, randomness, scheduling, external bindings, resource tracking, and GC coordination.
VM and native are "engines" that run until they yield back to the runtime (microtask-style).
Runtime behavior is modeled along three basic dimensions:

| Dimension | Values | Purpose |
|-----------|--------|---------|
| engine | `vm`, `native` | chooses the execution engine |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior |
| world | `host`, `simulation` | chooses host-backed or simulation-backed bindings |

## Components

| Component | Description |
|-----------|-------------|
| runtime | primary runtime instance and orchestration |
| scheduler | event loop, tasks, microtasks, and runnables |
| bindings | bindings, dispatch, and rule enforcement |
| time | virtual, monotonic, and wall clocks |
| random | deterministic streams and entropy control |
| memory | heap coordination and GC safepoints |
| replay | log schema, record, replay, and branching |
| snapshot | checkpoint capture and restore |
| platform | OS ("platform") integration and resources |
| telemetry | profiling and tracing |

## Scheduler

We follow WHATWG (and Node) semantics for event loop scheduling.
The unit of scheduling is a task, which represents a macrotask in the event loop.
A microtask represents a Promise job and is drained after each task.

Tasks and microtasks carry a runnable and a resume value.
The runnable can be a VM continuation or a native continuation, and the scheduler treats them uniformly.
(This keeps one scheduling model for both VM and native execution.)

VM entry points are single threaded per isolate, and only run one "tick" at a time.
The runtime decides when to run, yield, and resume, and it owns scheduling behavior.

## Bindings

All external effects ("platform effects") go through runtime bindings.
Platform code implements the actual OS integration and resource stuff.
Blocking work yields through the scheduler and resumes through the same bindings as everything else.

Binding dispatch is driven by binding scope.
Scope is declared per binding descriptor metadata, and generator validation enforces one effective scope per module.

## Modules

Every platform module follows one canonical layout.
Generated files (`*.generated.rs`) define the control plane and handwritten files define implementation adapters.

| Location | Responsibility |
|-----------|--------|
| `abi.generated.rs` | generated ABI types and encode or decode helpers for native and VM paths |
| `bindings.generated.rs` | generated registration, decode or encode glue, replay wrappers, and scope or world dispatch |
| `native.rs` | native engine adapter entrypoints for host scope |
| `vm.rs` | VM engine adapter entrypoints for host scope |
| `core.rs` | optional shared implementation helpers with no world routing |
| `tests/` | shared harness tests that run both engine adapters against the same binding contracts |

Modules with `host` bindings get the host and simulation backend folders:

| File or folder | Responsibility |
|-----------|--------|
| `host.rs` | cfg routing only: `unix`, `windows`, `unsupported` re-exports |
| `unix/` | host world Unix implementations |
| `windows/` | host world Windows implementations |
| `unsupported.rs` | host world fallback for unsupported targets |
| `simulation/mod.rs` | simulation backend wiring |
| `simulation/native.rs` | simulation backend native adapter surface |
| `simulation/vm.rs` | simulation backend VM adapter surface |

Modules with `runtime` bindings add runtime backend adapters.
These modules use the following extra files.

| File or folder | Responsibility |
|-----------|--------|
| `runtime/mod.rs` | runtime backend wiring |
| `runtime/native.rs` | runtime backend native adapter surface |
| `runtime/vm.rs` | runtime backend VM adapter surface |

Modules are scope-pure at the module level.
One module is either `host` scoped or `runtime` scoped, never both.
The generator enforces this.

### Dispatch

World and execution dispatch belongs to generated wrappers only

| Decision | Owner |
|-----------|--------|
| decode or encode ABI values | `bindings.generated.rs`, then adapter helpers in `vm.rs` or `native.rs` |
| execution mode (`fast`, `deterministic`, `record`, `replay`) | `bindings.generated.rs` replay wrapper |
| world (`host` or `simulation`) | `bindings.generated.rs` for `host` bindings only |
| host OS target (`unix`, `windows`, `unsupported`) | `host.rs` cfg routing |
| shared semantic helper logic | `core.rs` |

All bindings follow one stage pipeline.
The exact backend target depends on scope.

1. entry at generated exported wrapper in `bindings.generated.rs`.
2. decode args, validate pointers, and run rule and dispatch checks.
3. run replay gate for the active execution mode.
4. dispatch backend by scope and world.
5. run backend adapter and implementation.
6. encode result and return status.

Scope specific backend calls are listed below.

| Scope | VM engine | Native engine |
|-----------|--------|--------|
| `runtime` | `bindings.generated.rs -> runtime/vm.rs -> runtime subsystem or core helpers` | `bindings.generated.rs -> runtime/native.rs -> runtime subsystem or core helpers` |
| `host` | `bindings.generated.rs -> resolve world -> vm.rs or simulation/vm.rs -> host backend or simulation backend` | `bindings.generated.rs -> resolve world -> native.rs or simulation/native.rs -> host backend or simulation backend` |

Flow chart by scope is listed below.

```text
runtime scope:
  bindings.generated.rs
    -> runtime/{vm,native}.rs
      -> runtime subsystem implementation

host scope:
  bindings.generated.rs
    -> resolve world (host|simulation)
      -> {vm,native}.rs or simulation/{vm,native}.rs
        -> host.rs cfg route or simulation backend
```

The platform module scope matrix is listed below.
Counts come from `bindings.generated.rs` and represent unique binding descriptors per module.
Scope is declared per binding descriptor in builtin metadata.
Generator validation enforces one effective scope per module.

| Module | Description | `host` | `runtime` | Total |
|-----------|--------|--------|--------|--------|
| `audio` | Audio device and stream operations. | 28 | 0 | 28 |
| `crypto` | Cryptographic primitives and key operations. | 18 | 0 | 18 |
| `debug` | Debugger, tracing, profiling, and inspector hooks. | 0 | 11 | 11 |
| `display` | Display surfaces, modes, and presentation control. | 11 | 0 | 11 |
| `error` | Runtime error bridge and conversion helpers. | 0 | 1 | 1 |
| `ffi` | Dynamic libraries, symbols, and foreign calls. | 7 | 0 | 7 |
| `fs` | Filesystem paths, metadata, and file or directory operations. | 116 | 0 | 116 |
| `gpu` | GPU devices, queues, resources, and command submission. | 132 | 0 | 132 |
| `input` | Input devices, events, and state queries. | 43 | 0 | 43 |
| `io` | Generic host I/O primitives and descriptors. | 36 | 0 | 36 |
| `ipc` | Interprocess communication channels and message transfer. | 21 | 0 | 21 |
| `memory` | Runtime memory controls and host memory integration. | 14 | 0 | 14 |
| `net` | Sockets, addresses, protocols, and network I/O. | 102 | 0 | 102 |
| `os` | Operating system identity and host environment data. | 10 | 0 | 10 |
| `process` | Process identity, spawn, wait, signals, and limits. | 80 | 0 | 80 |
| `random` | Secure entropy and deterministic random streams. | 0 | 13 | 13 |
| `resource` | Runtime resource table and handle lifecycle management. | 0 | 4 | 4 |
| `security` | Policy, capability checks, and security controls. | 0 | 11 | 11 |
| `thread` | Thread local state, spawn, sync, and priority controls. | 30 | 0 | 30 |
| `time` | Clocks, timestamps, and time source access. | 0 | 20 | 20 |
| `tls` | Transport security sessions and certificate paths. | 20 | 0 | 20 |
| `tty` | TTY mode, capabilities, and terminal controls. | 8 | 0 | 8 |

Runtime scope ignores the world dimension.
Only `host` scope branches on world.

Execution mode behavior inside the replay gate is listed below.

| Execution mode | Replay gate behavior |
|-----------|--------|
| `fast` | executes backend directly and does not persist replay payload |
| `deterministic` | executes backend with deterministic providers and no replay payload |
| `record` | executes backend and persists replay payload |
| `replay` | rehydrates replay payload and bypasses backend execution |

### Testing

Binding tests assert binding level behavior, not adapter internals.
Host modules with non-trivial bindings should include a `tests/` harness.

The required test matrix is listed below.

| Scope | Required harness coverage |
|-----------|--------|
| `host` | run the same contract tests against `native` and `vm` adapters in host world |
| `runtime` | run the same contract tests against `runtime/native` and `runtime/vm` adapters |

Simulation backends may start as `notSupported` stubs.
When simulation implementations become real, add parity tests against host behavior where semantics are shared.

Path encoding is explicit at the binding boundary through `OsPath`.
On unix targets, `OsPath.Utf16` inputs are transcoded to UTF-8 bytes before syscall dispatch.
If a unix syscall returns path bytes that are not valid UTF-8, UTF-16 output paths fail loudly instead of lossy conversion.

Socket message bindings surface systems-level metadata directly.
`recvMsg` returns raw recv flags plus explicit payload and control truncation booleans.
`sendMsg` accepts raw send flags and validates them before syscall dispatch.

`sendfile` is target-aware.
Linux, Android, macOS, and iOS use kernel sendfile paths.
Other targets use a buffered copy fallback so behavior remains available.

## Rules, Effects, and Faults

Runtime options expose one ordered `rules` list.
Rules are evaluated in declaration order with first-match semantics at each decision site.
Each rule has one `when` filter and one `action`.
`SetWorld`, `SetAccess`, and `SetReplay` actions update binding routing and policy behavior.
`Fault` and `Control` actions inject runtime behavior changes.
Rules with `Fault` or `Control` actions may optionally declare `on` to target one hook such as `bindingBefore` or `schedulerDequeue`.

Fault effects are split into two categories.
Operation-level faults apply at binding boundaries, such as runtime delay, runtime error, and runtime timeout.
Component-level faults apply inside subsystem logic, such as net route partition or scheduler timer skew.

Host world supports operation-level faults directly in wrappers.
Host world supports component-level faults only when the target subsystem has explicit host-side injection points.
Simulation world supports both categories through `SimulationState` once the corresponding simulation subsystems are implemented.

## Determinism and Replay

Determinism is enforced by the runtime by controlling time, randomness, scheduling, and all other effects, and capturing all that in the "replay log" (when needed).
There are things we cannot replay, like userland FFI bindings or security-sensitive bindings, in which case we just error.

### Randomness

Randomness is split into deterministic "streams" so concurrent work does not cross-contaminate.
Each task and microtask gets a stable stream id, and user code can allocate its own stream ids explicitly.
Stream allocation itself is recorded as a replay event, so replays stay aligned even when streams are created dynamically.

### Snapshots

Snapshots capture heap state, task queues, clock state, random state, and resource mappings.
Snapshots are taken at safepoints where VM and native frames are in a resumable state.
Snapshots allow fast replay and time travel without re-executing from genesis.

## Testing

Run these from the repository root.

### Quick local loop

```sh
cargo test -p destack_runtime
```

### Platform coverage

```sh
just language/test-runtime-privileged
just language/test-windows-runtime
```
