# Runtime

Runtime library for WASM and native Destack binaries.
Provides scheduling, I/O, external bindings, snapshotting, and debug support.

# Scope

The runtime is **only** for WASM and native targets.
JS/TS targets use external runtimes (Node, Bun, Deno, browsers) and do not link this library.
The target split is simple:

| Target | Pipeline | Runtime |
| --- | --- | --- |
| JS/TS | Source -> Compiler -> JS/TS | External runtime (Node/Bun/Deno/browsers) |
| WASM/Native | Source -> Compiler -> Codegen -> Binary | Destack runtime |

The runtime is linked into compiled binaries; it is not a separate process.

# Objectives

The runtime provides the platform envelope around VM and native execution.
It must:

- provide a complete execution environment for MIR and native code
- enable Go-style concurrency with deterministic test control
- support snapshots, profiling, and full debug workflows
- preserve VM/native semantic equivalence

# Responsibilities

The runtime owns platform integration and orchestration around the VM or native code.
This includes:

- scheduling: event loop, timers, task queues
- I/O and bindings: filesystem, network, crypto, OS integration
- external handles: lifetime management and optional finalizers
- snapshotting: persistence format and serialization
- VM <-> native transitions: OSR, deopt, tiering decisions
- determinism controls: time, randomness, scheduling policy
- observability: profiling, tracing, debug UX aggregation

# VM Contract

The VM provides a complete MIR execution engine with continuations and GC.
The runtime drives it via:

- `run_function[_yielding]`, `resume(Continuation, Value)`
- `collect_garbage_with_continuations(&[Continuation])`
- profiling and trace hooks

The runtime supplies:

- extra GC roots for suspended continuations and external handles
- scheduling policy for yield/resume
- external handle resolution and native bindings
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

# Execution Policy

Execution policy settings are defined in target configuration and must be honored by the runtime for native targets.

**Tiering model** is fixed to VM + optimized native.
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

**Determinism** follows `determinismMode`, including scheduler determinism and record or replay I/O.
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

# External Handles

External resources are represented as opaque handles in the VM.
The runtime owns a handle table and optional finalizers.
Handles are rooted via runtime hooks when GC runs.

```ds
type ExternalHandle = {
    typeId: uint64,
    payloadPtr: *unknown,
    finalizer?: (handle: ExternalHandle) => void,
};

const handleId: uint64 = 0;
const handle = handleTable[handleId];
```

Finalizers run only at safe points and must not re-enter the VM.
Handles are excluded from snapshots unless the runtime provides serialization hooks.

# External Bindings

External functions must be non-reentrant with respect to the VM.
Blocking or asynchronous work is modeled in MIR as a yield/resume state machine.
External bindings should return immediately and signal completion by scheduling a resume value into the task queue.
External bindings are modeled as effects with typed payloads defined by the standard library and runtime.
Capability tokens are passed explicitly to authorize external effects.

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

External handles are excluded by default and require explicit runtime opt-in.
If a snapshot is requested with external handles present and no serialization hook, the runtime must fail loudly.
Any external handle must be reattached explicitly by the runtime after restore.
Snapshots are versioned and tied to the target ABI.
The runtime must reject snapshot restore when the compiler version, target triple, or GC layout does not match.

# VM <-> Native Transitions

Tiering relies on lowering metadata:

- Deopt maps reconstruct MIR frames from native registers and stack.
- OSR maps enter native code from MIR block boundaries.
- GC stack maps identify live managed references in native frames.

The runtime decides when transitions occur; correctness lives in the metadata.
Metadata formats are compiler-version private; mismatched versions must be rejected.

## Runtime ABI

NOTE #Incomplete #ABI: this ABI surface is defined but will be tightened as runtime shims and native transitions are finalized.

The runtime interacts with the VM through a strict entrypoint table.
The entrypoint table is per isolate.

```ds
type VmEntrypoints = {
    runFunction: (functionId: uint32, args: Value[]) -> VmOutcome,
    runFunctionYielding: (functionId: uint32, args: Value[]) -> VmOutcome,
    resume: (continuation: Continuation, resumeValue: Value) -> VmOutcome,
    collectGarbage: (extraRoots: Continuation[]) -> void,
};

type VmOutcome = VmYielded | VmCompleted | VmTrapped;

type VmYielded = {
    kind: 'yielded',
    continuation: Continuation,
};

type VmCompleted = {
    kind: 'completed',
    value: Value,
};

type VmTrapped = {
    kind: 'trapped',
    error: VmError,
};

type VmError = {
    code: uint16,
    detail: string,
};
```

The runtime must validate that entrypoints are not reentrant for a given isolate.
The runtime must serialize all entrypoint calls per isolate.

## Native Transition ABI

Native code transfers control to the VM using a deopt entrypoint.
The deopt entrypoint takes the native machine state and a deopt reason.

```ds
type NativeState = {
    pc: uint64,
    framePointer: uint64,
    stackPointer: uint64,
    registers: uint64[],
};

type DeoptReason = {
    code: uint16,
    siteId: uint32,
};

type DeoptRequest = {
    deoptMap: uint32,
    state: NativeState,
    reason: DeoptReason,
};
```

The runtime must ensure `deoptMap` is a valid index into the metadata blob.

## External Shim ABI

External I/O flows through runtime shims in deterministic modes.
Each external call is represented as a typed effect with an opaque payload.

```ds
type ExternalCall = {
    effectId: uint32,
    payload: uint8[],
};

type ExternalResult = {
    payload: uint8[],
    errorCode: uint16,
};
```

`Record` captures ExternalCall and ExternalResult pairs in program order.
`effectId` refers to a runtime effect registry with stable payload schemas.

# Determinism

Deterministic execution requires the runtime to control:

- Wall clock and time sources are controlled.
- Randomness is controlled via seeded PRNG.
- Scheduling policy and preemption points are controlled.

Determinism follows `determinismMode`:
- `Deterministic` fixes task ordering, preemption decisions, and PRNG seeds.
- `Record` records external I/O for deterministic playback.
- `Replay` replays external I/O from the runtime log.

Deterministic I/O is opt-in and requires runtime support for record and replay.
Record and Replay enable time-travel debugging and simulation testing in userland libraries.

## Record/Replay Log

The runtime records external effects in program order.
Each entry captures the effect identifier and its payload.
Each entry captures the result payload or error code.

```ds
type ReplayEntry = {
    sequence: uint64,
    call: ExternalCall,
    result: ExternalResult,
};
```

`sequence` is a monotonically increasing identifier assigned by the runtime.

# Observability

The runtime aggregates VM/native signals:

- Counters and sampling from the VM are aggregated.
- Timeline trace events across VM and native are aggregated.
- Debug inspection tools and source mapping are aggregated.
Telemetry libraries should join VM and native signals using the shared trace timeline.
