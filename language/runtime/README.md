# Runtime

Runtime library for WASM and native Destack binaries.
Provides scheduling, I/O, external bindings, snapshotting, and debug support.

# Scope

The runtime is **only** for WASM and native targets.
JS/TS targets use external runtimes (Node, Bun, Deno, browsers) and do not link this library.
The runtime is linked into compiled binaries; it is not a separate process.

## Components

| Component | Description |
|-----------|-------------|
| `platform` | bindings, clock, random, resources |
| `scheduler` | tasks, event loop, timers |
| `replay` | record and replay log for deterministic I/O |
| `snapshot` | persistence format and restore |
| `telemetry` | trace and profiling aggregation |
| `bridge` | VM entrypoints and isolate orchestration |

Determinism is defined by scheduler policy, platform sources, and replay.

## Native Binding ABI

Native code calls runtime bindings through a typed C ABI.
The runtime provides a TLS-backed host call context that carries the active host state and binding policy.

The native ABI uses simple FFI-safe structs:

- `HostStringRef`: `{ data: *const u8, len: u64 }` UTF-8 string view
- `HostStringSlice`: `{ data: *const HostStringRef, len: u64 }` slice of strings
- `HostStatus`: `{ code: u32 }` status code (0 = success)

Bindings are exported under stable names like `destack.console.log` and `destack.process.args`.
The runtime enforces binding policy (determinism and replay) at the ABI boundary.
VM execution uses `ExternalContext` and `Value` shims instead of this typed ABI.

# Responsibilities

The runtime owns platform integration and orchestration around the VM or native code.
This includes:

- scheduling: event loop, timers, task queues
- I/O and bindings: filesystem, network, crypto, OS integration
- external resources: lifetime management and optional finalizers
- snapshotting: persistence format and serialization
- VM <-> native transitions: OSR, deopt, tiering decisions
- determinism controls: time, randomness, scheduling policy
- telemetry: profiling, tracing, debug UX aggregation

## Security and Trust Policy

The runtime defines a trust policy for each isolate.
This policy is part of the target configuration and must be enforced consistently.
The sandbox policy is part of the target configuration as well.

Untrusted execution requires:
- strict VM limits and runtime checks
- protected or forbidden external calls
- all I/O through runtime shims
- validation of all VM entrypoints and continuations

Trusted execution may relax limits for internal code but keeps validation and telemetry.
Compiled execution is allowed only for code produced by the toolchain with verified metadata.
External native binaries must run under OS sandboxing or be rejected.

Sandboxing levels are deployment-specific:
- in-process isolation with VM guardrails
- process isolation with rlimits and OS sandboxing
- container or VM isolation for cloud providers

# VM Contract

The VM provides a complete MIR execution engine with continuations and GC.
The runtime drives it via:

- `run_function[_yielding]`, `resume(Continuation, Value)`
- `collect_garbage_with_continuations(&[Continuation])`
- profiling and trace hooks

The runtime supplies:

- extra GC roots for suspended continuations and external resources
- scheduling policy for yield/resume
- external resource resolution and native bindings
- serialization of VM entry point calls per isolate

The runtime must treat continuations as opaque values and only pass them back to the VM.
Continuations are single-shot unless explicitly forked by the VM API.

## Debug Execution Mode

Debug workflows are controlled by `debugMode` in target configuration.
The runtime must honor this mode when selecting VM vs native execution:

| Variant | Behavior |
|---------|----------|
| `Auto` | Use `Deopt` for debug builds, `Native` for release |
| `Vm` | Force interpreter execution |
| `Deopt` | Run native with deopt-first debugging |
| `Native` | Run native only (no deopt) |

When `debugMode = Deopt`, the runtime must reject binaries missing full deopt metadata.

## Execution Scenarios

| Scenario | Runtime behavior | Notes |
|----------|------------------|-------|
| VM-first JIT | start in the interpreter, compile hot functions, OSR into compiled | uses profiling and tiering |
| AOT-first debug | run compiled code, deopt on debug events | requires full deopt metadata |
| AOT-first | run compiled code only | debug off, deopt optional |
| Mixed debug | force selected code to the VM, compiled elsewhere | used for partial debugging |
| VM-only | interpreter only, compiled disabled | comptime or deterministic runs |
| Replay run | replay log drives external bindings | deterministic playback |
| Compiled-only slim runtime | compiled only, interpreter not linked | minimal runtime footprint |
| Comptime-only | not a runtime mode | VM executes during compilation |

## Hot Reload Integration

Hot reload is implemented as a runtime protocol and a library contract.
The runtime supports module restart and optional state preservation hooks.
Library code can register reload boundaries and state adapters for safe reloads.
State preserving reload is opt in and may be restricted by target policy.

# Execution Policy

Execution policy settings are defined in target configuration and must be honored by the runtime for native targets.

**Tiering model** is fixed to VM + compiled code.
The runtime decides when to enter native code and when to deopt back to the VM.

**Profiling** follows `profilingMode`.
Profiling signals are scoped per isolate and consumed by optimizer/tiering logic.

Profiling signals include:

- Call counts per function and callsite.
- Loop backedge counts.
- Allocation counts and bytes per site.
- Guard failures and deopt reasons.
- Branch direction bias per conditional branch.
- Indirect call target counts for interface dispatch.
- PC sampling for hot instruction ranges when sampling is enabled.

The runtime may forward these signals to telemetry pipelines, but collection points are defined by the compiler metadata.
Telemetry libraries should consume these signals rather than re-instrumenting hot paths.

**Speculation** follows `speculationMode` and requires guard metadata and deopt maps.
The runtime must surface deopt reasons for profiling and diagnostics.

**OSR** follows `osrMode` and can be disabled or restricted to explicit sites.
Tiering triggers are tunable and include backedge counts, wall-clock time, allocation pressure, and explicit `@hot` or `@cold` hints.

**Safepoints** follow `safepointMode` and may include instruction-budget polling via `safepointInterval`.
The interval is a step budget.
The VM decrements the budget per threaded instruction.
Native code decrements it at inserted safepoint polls.

**Determinism** follows `determinism`, including scheduler determinism and controlled randomness.
**Replay** follows `replay` and controls record or replay I/O.
`Record` and `Replay` require all external calls to go through runtime shims; unshimmed FFI/syscalls must be rejected.

External I/O is any operation outside the VM interpreter.
Filesystem and network access are external I/O.
Process, environment, and clock sources are external I/O.
Randomness and entropy sources are external I/O.
Host callbacks and FFI calls are external I/O.

**GC strategy** is Go-style, headerless, and generational by default.
The runtime coordinates safepoints and may enable incremental marking.
Deterministic modes must use deterministic GC scheduling based on allocation thresholds.
The barrier model is Go-style Dijkstra with shade-on-write.
GC runs inside the VM; compiled code participates via stack maps at safepoints.

# Scheduling Model

We target Go-style concurrency: many tasks per OS thread with cooperative preemption.
The runtime requests preemption and the VM or native code yields at safepoints.
Safepoints include backedges, call sites, and allocation points.
Structured concurrency primitives in the standard library map to task metadata and scheduler hints.

## Task Model

A task is a continuation plus a resume payload and scheduling metadata:

```ds
type TaskState = 'ready' | 'waiting' | 'completed';

type Task = {
    continuation: Continuation,
    resumeValue: Value,
    state: TaskState,
    priority: uint8,
};
```

Tasks are single-threaded within an isolate.
Cross-isolate scheduling is runtime-defined.

## Scheduler Loop

```ds
loop {
    const task = scheduler.next();
    const outcome = vm.resume(task.continuation, task.resumeValue);
    match (outcome) {
        Yielded(yielded) => scheduler.enqueue(task)
        Completed(output) => finalize(task)
    }
}
```

## Preemption

Preemption is cooperative.
The runtime requests preemption and the VM or native code yields at safepoints.
The runtime must not preempt inside a VM entry point.

## Isolate Lifecycle

Isolates are created with explicit resource budgets (heap, stack, step limits).
The runtime is responsible for:

- Serializing VM entry points per isolate.
- Supplying deterministic time and randomness sources.
- Draining queues and running finalizers on shutdown.

# Resources

External resources are represented as opaque resource ids in the VM.
The runtime owns a resource table and optional finalizers.
Resources are rooted via runtime hooks when GC runs.

```ds
type ExternalResource = {
    typeId: uint64,
    payloadPtr: *unknown,
    finalizer?: (resource: ExternalResource) => void,
};

const resourceId: uint64 = 0;
const resource = resourceTable[resourceId];
```

Finalizers run only at safe points and must not re-enter the VM.
Resources are excluded from snapshots unless the runtime provides serialization hooks.

# Bindings

External functions must be non-reentrant with respect to the VM.
Blocking or asynchronous work is modeled in MIR as a yield/resume state machine.
Bindings should return immediately and signal completion by scheduling a resume value into the task queue.
Bindings are modeled as typed external calls defined by the standard library and runtime.
Capability tokens are passed explicitly to authorize external bindings.

The runtime must surface failures as userland values rather than raising VM-level exceptions from within external callbacks.
Synchronous externals are allowed when they are fast and non-blocking.
Externally driven callbacks must not call into the VM while it is running.

# Snapshotting

Snapshots capture VM state plus runtime-defined scheduling state.
Snapshots are required to implement `Record` and `Replay`.
Snapshots are a core debugging tool for deterministic execution.

A snapshot captures:

- Managed heap, raw heap, globals, and continuation queues are captured.
- Deterministic sources such as time epoch and PRNG seed are captured.
- Runtime scheduler queues and timers are captured.

External resources are excluded by default and require explicit runtime opt-in.
If a snapshot is requested with external resources present and no serialization hook, the runtime must fail loudly.
Any external resource must be reattached explicitly by the runtime after restore.
Snapshots are versioned and tied to the target ABI.
The runtime must reject snapshot restore when the compiler version, target triple, or GC layout does not match.

# Replay and Determinism

Deterministic execution requires the runtime to control:

- Wall clock and time sources are controlled.
- Randomness is controlled via seeded PRNG.
- Scheduling policy and preemption points are controlled.

Determinism follows `determinism`:
- `Deterministic` fixes task ordering, preemption decisions, and PRNG seeds.

Replay follows `replay`:
- `Record` records external I/O for deterministic playback.
- `Replay` replays external I/O from the runtime log.

Deterministic I/O is opt-in and requires runtime support for replay.
Record and Replay enable time-travel debugging and simulation testing in userland libraries.

Determinism requires routing all external bindings through runtime shims.
Time, randomness, and scheduling are part of the deterministic surface.

## Replay Log

The runtime records external bindings in program order.
Each entry captures the binding identifier and its payload.
Each entry captures the result payload or error code.

```ds
type ReplayEntry = {
    sequence: uint64,
    call: ExternalCall,
    result: ExternalResult,
};
```

`sequence` is a monotonically increasing identifier assigned by the runtime.