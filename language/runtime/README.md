# Runtime

The runtime is how Destack actually does anything meaningful beyond pure computation.
It wraps VM and/or native execution with scheduling, bindings, determinism, replay, snapshots, and policy.
Essentially, the runtime is where we marry Node/Bun/Deno-level semantics _with_ V8/JSC-runtime features, all in one place.

## Overview

Destack has a single runtime that can drive both VM and native execution (even simultaneously).
The runtime owns time, randomness, scheduling, external bindings, resource tracking, and GC coordination.
VM and native are "engines" that run until they yield back to the runtime (microtask-style).

## Dimensions

Runtime behavior is modeled along three orthogonal dimensions.

| Dimension | Values | Purpose |
|-----------|--------|---------|
| engine | `vm`, `native` | chooses the execution engine |
| execution | `fast`, `deterministic`, `record`, `replay` | chooses determinism and replay behavior |
| world | `host`, `simulated` | chooses host-backed or simulated bindings |

## Components

The runtime is organized around subsystems that will sound familiar to V8 and JSC enjoyers, with some additional features for Destack:

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

## Execution Model

We follow traditional WHATWG (and Node) semantics for event loop scheduling.
The unit of scheduling is a task, which represents a macrotask in the event loop.
A microtask represents a Promise job and is drained after each task.

Tasks and microtasks carry a runnable and a resume value.
The runnable can be a VM continuation or a native continuation, and the scheduler treats them uniformly.
(This keeps one scheduling model for both VM and native execution.)

VM entry points are single threaded per isolate, and only run one "tick" at a time.
The runtime decides when to run, yield, and resume, and it owns the scheduling policy.

## Bindings and Platform

All external effects ("platform effects") go through runtime bindings.
Platform code implements the actual OS integration and resource stuff.
Blocking work yields through the scheduler and resumes through the same bindings as everything else.

OS-backed bindings are split into `host` and `simulated` implementations.
The generated binding wrappers resolve policy, then dispatch to `host::*` or `simulated::*`.
Runtime-backed bindings do not use an extra host/simulated split and instead defer to runtime subsystem behavior.

Module layout follows binding scope.
Modules with `os` or `hybrid` bindings use host world routing and simulated world routing.
They keep generated wrappers at the top level and place host OS backend routing in `host.rs`.
They place simulated stubs and implementations in `simulated/`.
Modules with `runtime`-only bindings do not add a host/simulated backend split.
They keep behavior in runtime subsystems and use `native.rs` and `vm.rs` as engine adapters.
Mixed modules apply both rules per binding scope.

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

Rules are evaluated in declaration orders, first match wins, with filters like (binding glob, capability glob, component glob, module glob, engine, execution mode, platform, scope, blocking class, and effect class).

Fault effects are split into two categories:
 - Operation-level faults apply at binding boundaries, such as runtime delay, runtime error, and runtime timeout.
 - Component-level faults apply inside subsystem logic, such as net route partition or scheduler timer skew.

Host world supports operation-level faults directly in wrappers.
Host world supports component-level faults only when the target subsystem has explicit host-side injection points.
Simulated world supports both categories fully through `SimulationState` and deterministic schedulers.

## Determinism and Replay

Determinism is enforced by the runtime by controlling time, randomness, scheduling, and all other effects, all of which is captured in the "replay log" (when needed).
As the name implies, the replay log records everything we need to replay the program execution deterministically.
(There are things we cannot replay, in which case we just error. Sad.)

## Randomness

Randomness is split into deterministic "streams" so concurrent work does not cross-contaminate.
Each task and microtask gets a stable stream id, and user code can allocate its own stream ids explicitly.
Stream allocation itself is recorded as a replay event, so replays stay aligned even when streams are created dynamically.

## Snapshots

Snapshots capture heap state, task queues, clock state, random state, and resource mappings.
Snapshots are taken at safepoints where VM and native frames are in a resumable state.
Snapshots allow fast replay and time travel without re-executing from genesis.
