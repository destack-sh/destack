# Runtime

The runtime is how Destack actually does anything meaningful beyond pure computation.
It wraps VM and/or native execution with scheduling, bindings, record/replay, and policy checks.
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
| world | `host`, `simulated` | chooses host-backed or simulated bindings |

## Components

| Component | Description |
|-----------|-------------|
| runtime | primary runtime instance and orchestration |
| scheduler | event loop, tasks, microtasks, and runnables |
| bindings | bindings and effects |
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
The runtime decides when to run, yield, and resume, and it owns the scheduling policy.

## Bindings

All external effects ("platform effects") go through runtime bindings.
Platform code implements the actual OS integration and resource stuff.
Blocking work yields through the scheduler and resumes through the same bindings as everything else.

Binding dispatch is driven by binding scope.
Scope is per binding descriptor, not per module.

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

Modules with `os` or `hybrid` bindings get the host and simulated backend folders:

| File or folder | Responsibility |
|-----------|--------|
| `host.rs` | cfg routing only: `unix`, `windows`, `unsupported` re-exports |
| `unix/` | host world Unix implementations |
| `windows/` | host world Windows implementations |
| `unsupported.rs` | host world fallback for unsupported targets |
| `simulated/mod.rs` | simulated backend wiring |
| `simulated/native.rs` | simulated backend native adapter surface |
| `simulated/vm.rs` | simulated backend VM adapter surface |

Modules with `runtime` bindings add runtime backend adapters.
These modules use the following extra files.

| File or folder | Responsibility |
|-----------|--------|
| `runtime/mod.rs` | runtime backend wiring |
| `runtime/native.rs` | runtime backend native adapter surface |
| `runtime/vm.rs` | runtime backend VM adapter surface |

Mixed modules include both sets and route per binding scope.
The existence of `host.rs` or `simulated/` does not imply every binding in that module uses world dispatch.

### Dispatch

World and execution dispatch belongs to generated wrappers only

| Decision | Owner |
|-----------|--------|
| decode or encode ABI values | `bindings.generated.rs`, then adapter helpers in `vm.rs` or `native.rs` |
| execution mode (`fast`, `deterministic`, `record`, `replay`) | `bindings.generated.rs` replay wrapper |
| world (`host` or `simulated`) | `bindings.generated.rs` for `os` or `hybrid` bindings only |
| host OS target (`unix`, `windows`, `unsupported`) | `host.rs` cfg routing |
| shared semantic helper logic | `core.rs` |

All bindings follow one stage pipeline.
The exact backend target depends on scope.

1. entry at generated exported wrapper in `bindings.generated.rs`.
2. decode args, validate pointers, and check policy.
3. run replay gate for the active execution mode.
4. dispatch backend by scope and world.
5. run backend adapter and implementation.
6. encode result and return status.

Scope specific backend calls are listed below.

| Scope | VM engine | Native engine |
|-----------|--------|--------|
| `runtime` | `bindings.generated.rs -> runtime/vm.rs -> runtime subsystem or core helpers` | `bindings.generated.rs -> runtime/native.rs -> runtime subsystem or core helpers` |
| `os` | `bindings.generated.rs -> resolve world -> vm.rs or simulated/vm.rs -> host backend or simulation backend` | `bindings.generated.rs -> resolve world -> native.rs or simulated/native.rs -> host backend or simulation backend` |
| `hybrid` | `bindings.generated.rs -> resolve world -> vm.rs or simulated/vm.rs -> host plus runtime mixed backend` | `bindings.generated.rs -> resolve world -> native.rs or simulated/native.rs -> host plus runtime mixed backend` |

Flow chart by scope is listed below.

```text
runtime scope:
  bindings.generated.rs
    -> runtime/{vm,native}.rs
      -> runtime subsystem implementation

os scope:
  bindings.generated.rs
    -> resolve world (host|simulated)
      -> {vm,native}.rs or simulated/{vm,native}.rs
        -> host.rs cfg route or simulation backend

hybrid scope:
  bindings.generated.rs
    -> resolve world (host|simulated)
      -> {vm,native}.rs or simulated/{vm,native}.rs
        -> host backend and runtime subsystem helpers as needed
```

The platform module scope matrix is listed below.
Counts come from `bindings.generated.rs` and represent unique binding descriptors per module.
Scope is per binding descriptor, not per module.
Modules may include bindings from more than one scope.

| Module | Description | `os` | `hybrid` | `runtime` | Total |
|-----------|--------|--------|--------|--------|--------|
| `audio` | Audio device and stream operations. | 10 | 0 | 0 | 10 |
| `console` | Console and terminal text I/O. | 4 | 0 | 0 | 4 |
| `crypto` | Cryptographic primitives and key operations. | 18 | 0 | 0 | 18 |
| `debug` | Debugger, tracing, profiling, and inspector hooks. | 0 | 0 | 11 | 11 |
| `device` | Host device discovery and control. | 5 | 0 | 0 | 5 |
| `display` | Display surfaces, modes, and presentation control. | 11 | 0 | 0 | 11 |
| `error` | Runtime error bridge and conversion helpers. | 0 | 0 | 1 | 1 |
| `ffi` | Dynamic libraries, symbols, and foreign calls. | 3 | 0 | 4 | 7 |
| `fs` | Filesystem paths, metadata, and file or directory operations. | 116 | 0 | 0 | 116 |
| `gpu` | GPU devices, queues, resources, and command submission. | 46 | 0 | 0 | 46 |
| `input` | Input devices, events, and state queries. | 4 | 2 | 0 | 6 |
| `io` | Generic host I/O primitives and descriptors. | 26 | 0 | 0 | 26 |
| `ipc` | Interprocess communication channels and message transfer. | 21 | 0 | 0 | 21 |
| `memory` | Runtime memory controls and host memory integration. | 14 | 0 | 1 | 15 |
| `net` | Sockets, addresses, protocols, and network I/O. | 102 | 0 | 0 | 102 |
| `os` | Operating system identity and host environment data. | 10 | 0 | 0 | 10 |
| `process` | Process identity, spawn, wait, signals, and limits. | 66 | 0 | 11 | 77 |
| `random` | Secure entropy and deterministic random streams. | 0 | 3 | 10 | 13 |
| `resource` | Runtime resource table and handle lifecycle management. | 0 | 0 | 4 | 4 |
| `security` | Policy, capability checks, and security controls. | 0 | 1 | 10 | 11 |
| `thread` | Thread local state, spawn, sync, and priority controls. | 30 | 0 | 0 | 30 |
| `time` | Clocks, timestamps, and time source access. | 2 | 8 | 0 | 10 |
| `timer` | Runtime timers, scheduling, and timer descriptor APIs. | 5 | 0 | 10 | 15 |
| `tls` | Transport security sessions and certificate paths. | 20 | 0 | 0 | 20 |
| `tty` | TTY mode, capabilities, and terminal controls. | 8 | 0 | 0 | 8 |

Runtime scope ignores the world dimension.
Only `os` and `hybrid` scopes branch on world.

Execution mode behavior inside the replay gate is listed below.

| Execution mode | Replay gate behavior |
|-----------|--------|
| `fast` | executes backend directly and does not persist replay payload |
| `deterministic` | executes backend with deterministic providers and no replay payload |
| `record` | executes backend and persists replay payload |
| `replay` | rehydrates replay payload and bypasses backend execution |

### Testing

Binding tests assert binding level behavior, not adapter internals.
Host and hybrid modules with non-trivial bindings should include a `tests/` harness.

The required test matrix is listed below.

| Scope | Required harness coverage |
|-----------|--------|
| `os` | run the same contract tests against `native` and `vm` adapters in host world |
| `hybrid` | run the same contract tests against `native` and `vm` adapters in host world, then add targeted simulated world tests as implementations land |
| `runtime` | run the same contract tests against `runtime/native` and `runtime/vm` adapters |

Simulated backends may start as `notSupported` stubs.
When simulated implementations become real, add parity tests against host behavior where semantics are shared.

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

Rules are evaluated in declaration order (first match wins) with glob-style files for bindings, capabilities, arguments, etc. to apply "effects" that modify the runtime behavior in some way.

Fault effects are split into two categories:
 - Operation-level faults apply at binding boundaries, such as runtime delay, runtime error, and runtime timeout.
 - Component-level faults apply inside subsystem logic, such as net route partition or scheduler timer skew.

Host world supports operation-level faults directly in wrappers.
Host world supports component-level faults only when the target subsystem has explicit host-side injection points.
Simulated world supports both categories fully through `SimulationState` and deterministic schedulers.

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
