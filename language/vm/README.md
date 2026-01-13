# VM

MIR interpreter for comptime execution, debug mode, deoptimization, and native fallback.
The VM is a fully featured execution engine for MIR.

# Overview

## Objectives

The VM preserves native semantics while enabling a broader execution envelope:
- **comptime**: evaluate `comptime { }` blocks during compilation
- **debugging**: full introspection, breakpoints, and step control
- **deoptimization**: continue optimized native code in the VM
- **fallback execution**: run MIR directly when native code is unavailable

## Scope

The VM owns correctness, GC, and state reconstruction.
The runtime owns scheduling, I/O, and platform integration.

| Area | VM owns | Runtime owns |
|------|---------|--------------|
| Execution | MIR semantics, stacks, continuations | task queues, event loop |
| Memory | managed GC, raw heap, stack alloc | external resources |
| Transitions | state reconstruction | OSR/deopt policy |
| Telemetry | counters, profiling hooks | aggregation, UX |
| Snapshotting | state capture hooks | persistence format |

## Layout

<pre>
┌─────────────────────────────────────────────────────────────────────────────┐
│                                     VM                                      │
│                                                                             │
│  Isolate                                                                    │
│   ├─ heaps, GC, globals, interned strings                                   │
│   ├─ options, telemetry hooks                                               │
│   └─ externals registry (runtime-provided)                                  │
│                                                                             │
│  Engines                                                                    │
│   ├─ interpreter engine: threaded decode + dispatch                         │
│   └─ compiled engine: OSR, deopt, stack map consumption                      │
│                                                                             │
│  Execute                                                                    │
│   ├─ entrypoints: run_function, resume                                      │
│   └─ continuations: capture and restore                                     │
│                                                                             │
│  ABI (vm/abi)                                                               │
│   └─ metadata: safepoints, deopt maps, stack maps, versions                  │
└─────────────────────────────────────────────────────────────────────────────┘
</pre>

### Components

| Component | Description |
|-----------|-------------|
| `isolate` | per-instance state: heaps, globals, options, telemetry hooks |
| `engine::interpreter::InterpreterEngine` | threaded decode and MIR dispatch |
| `engine::compiled::CompiledEngine` | compiled execution (AOT or JIT), OSR, deopt, stack map consumption |
| `execute` | entrypoints and continuation state machine |
| `memory` | Value representation, managed heap, raw heap, GC |
| `diagnostic` | errors and stack traces |
| `telemetry` | counters and profiling hooks |
| `snapshot` | state capture and restore |
| `abi` (`vm/abi`) | metadata wire types and versioning |

### Ownership boundary

The VM executes MIR and reconstructs state.
Platform services (I/O, bindings, scheduling, replay) live in the runtime.

## Security Model

The VM is designed to run untrusted MIR safely when configured by the runtime.
Security is enforced by strict entrypoint validation, explicit host boundaries, and resource limits.

The runtime selects a trust policy per isolate:
- **Untrusted**: strict limits and checks, protected or forbidden externals, no unbounded execution
- **Trusted**: relaxed limits for internal code, still with validated entrypoints

The VM never performs syscalls or host I/O.
All external effects must go through runtime-provided externals and their policy.

Compiled entrypoints are only used when metadata is present and version-checked.
External native binaries must be sandboxed by the runtime or rejected.

## Usage Scenarios

| Scenario | Start | Transitions | Notes |
|----------|-------|-------------|-------|
| VM-first JIT | interpreter | OSR into compiled, deopt back | hot code is compiled on demand |
| AOT-first debug | compiled | deopt at safepoints | stepping and inspection in the VM |
| AOT-first | compiled | optional deopt | full-speed execution, debug off |
| Mixed debug | interpreter + compiled | selective deopt | debug modules in VM, others compiled |
| VM-only | interpreter | none | comptime, deterministic runs, fallback |
| Replay run | interpreter or compiled | replays external bindings | deterministic playback from log |
| Compiled-only slim runtime | compiled | deopt disabled | interpreter not linked |
| Comptime-only | interpreter | none | compile-time evaluation only |

## Terminology

| Term | Meaning |
|------|---------|
| isolate | one VM instance with its own heaps and globals |
| continuation | suspended execution state captured at `yield` |
| safepoint | program point where control can transfer VM <-> native |
| deopt map | metadata to reconstruct MIR state from native |
| OSR map | metadata to enter native code from MIR |
| stack map | metadata describing live GC references |

# Execution Model

## Isolate Model

A VM instance is a single isolate.
Each isolate owns its heaps and globals and is single-threaded at VM entry points.
The runtime may host many isolates across OS threads.

| State | Owner | Contents | Snapshot |
|-------|-------|----------|----------|
| isolate state | VM | managed heap, raw heap, globals, options | yes |
| execution state | VM | call stack, value stack, local stack | yes |
| continuation state | VM | suspended stacks + resume point | yes |
| runtime state | runtime | scheduler queue, timers, external resources | runtime-defined |

## Threading and Reentrancy

VM entry points are not reentrant and must be serialized per isolate.
A single isolate has exactly one mutator at a time (VM or native).
The runtime must not call into the VM from an external callback while the VM is running.

## Configuration

The isolate is configured by `IsolateOptions`:

| Group | Option | Meaning |
|-------|--------|---------|
| `execution` | `mode` | comptime/debug/deopt/runtime role |
| `policy` | `trust_policy` | untrusted/trusted/internal |
| `policy` | `runtime` | full/no-managed/no-runtime |
| `policy` | `borrow_mode` | hint or strict runtime enforcement |
| `policy` | `external_calls` | allow/protect/forbid externals |
| `checks` | `bounds` | bounds check policy |
| `checks` | `null` | null check policy |
| `checks` | `enforce_reference_kinds` | debug-only reference checks |
| `checks` | `enforce_reference_mutability` | debug-only mutability checks |
| `limits` | `max_stack_depth` | stack overflow limit |
| `limits` | `max_heap_cells` | managed heap budget in cells |
| `limits` | `max_raw_cells` | raw heap budget in cells |
| `limits` | `max_instructions` | step limit (timeout) |
| `telemetry` | `collect_stats` | enable execution statistics |

Heap budgets are specified in cells.
Each cell accounts for its header and slot storage.

## Execution Entry Points

The VM exposes a small, strict surface:

- `run_function`: run MIR to completion, error on yield
- `run_function_yielding`: run until completion or yield
- `resume(Continuation, Value)`: resume a suspended execution
- `collect_garbage_with_continuations(&[Continuation])`: GC with extra roots

The resume value carries effect results, errors, or cancellation as userland values.

### Entry Point ABI

NOTE #Incomplete #ABI: this entrypoint ABI is defined but will be tightened as the runtime contract is finalized.

```ds
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

The VM reports explicit errors for invalid entry usage:

- `run_function` yields: `UnexpectedYield`
- `resume` without a valid continuation: `InvalidContinuation`
- `resume` into a non-idle interpreter: `InvalidContinuation`

Yield leaves the interpreter idle with all mutable stacks moved into the continuation.
The runtime may reuse the interpreter only after yield or completion.

# Continuations

Yield suspends execution and returns a continuation.
Resume takes a single `Value` payload; error and cancellation are modeled in userland types.
Multi-shot resumption is explicit via `cloneForFork`.

## Continuation Contents

A continuation logically contains:

- the full call stack with per-frame metadata
- the SSA value stack and local stack
- the resume point (block + argument binding plan)
- execution statistics and profiling state

Continuations are opaque to the runtime.
The runtime must not inspect or mutate continuation internals.

## Continuation ABI

```ds
type FrameIndex = uint32;
type MirValueId = uint32;

type YieldState = {
    frameIndex: FrameIndex,
    resumeBlock: uint32,
    resumeCopies: CopyRange,
    resumeValue: MirValueId,
};

type Continuation = {
    isolateId: uint64,
    callStack: Frame[],
    valueStack: Value[],
    localStack: Value[],
    yieldState: YieldState,
    statistics: Statistics,
    instructionProfile?: InstructionProfile,
};
```

Frames follow these rules:

- `callStack[0]` is the bottom frame, `callStack.last()` is the current frame
- each frame owns a contiguous slice of `valueStack` and `localStack`
- `frame.valueBase + frame.valueCount` and `frame.localBase + frame.localCount`
  are within the stack bounds

### Resume Binding Rules

The resume block arguments are bound in two phases:

1. explicit resume arguments from the `yield` terminator
2. the resume payload appended after explicit arguments

If a resume argument is missing, the VM fills it with `Void`.
If the resume payload has no destination slot, it is ignored.

```mir
function @yield_prefix(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 10i32
    v2 = iconst 20i32
    yield v1, block1(v0, v2)
block1(v3: i32, v4: i32, v5: i32):
    v6 = iadd v3, v4
    v7 = iadd v6, v5
    return v7
}
```

### Single-shot and Forking

Continuations are single-shot by default.
After a resume, the original continuation is consumed and must not be reused.

`cloneForFork` copies the call stack, SSA values, locals, and stack cells.
Managed heap and globals are shared unless the runtime snapshots.
The runtime must not use multi-shot continuations for deterministic branching
without isolating heap/global state.

### Yield/Resume State Machine

```ds
// Running -> Yielded(Continuation) -> Running -> Completed
// Running -> Completed
```

Illegal transitions:
- resume without a valid continuation
- resume into a non-idle interpreter
- yield from a non-yielding entry point

# Safepoints and Preemption

Preemption is cooperative.
The runtime requests preemption and the VM or native code yields at safepoints.
Lowering inserts safepoints at:

- Call sites.
- Loop back-edges.
- Allocation points.
- Explicit `yield` terminators.

Native code must honor the same safepoint policy.

## Instruction Budget

`maxInstructions` limits the number of threaded instructions executed.
When the limit is reached, the VM returns `StepLimitExceeded`.

`safepointInterval` is a preemption budget, not a hard stop.
The VM decrements the safepoint budget per threaded instruction and checks it at safepoints.
A smaller interval improves responsiveness and determinism but adds overhead.

# Memory and GC

The VM manages three kinds of memory, matching MIR semantics:

| Kind | Instruction | Lifetime | Use Case |
|------|-------------|----------|----------|
| Managed | `managed.alloc` | GC-tracked | normal objects, TS semantics |
| Raw | `raw.alloc`/`raw.free` | manual | performance-critical, `^T` types |
| Stack | `stack.alloc` | frame-scoped | temporaries, small allocations |

The managed heap uses a precise tracing GC.
GC runs in the VM and scans compiled frames using stack maps at safepoints.
The raw heap tracks allocations for leak detection in debug builds.
Stack allocations are freed when frames exit.
The raw heap is not scanned by GC.
Storing managed references in raw allocations is undefined unless explicitly rooted by the runtime.
Non-generic address spaces may be modeled by the runtime or rejected by the VM with appropriate diagnostics.

## GC Root Set

Roots include:

- Active frames including SSA values, locals, and stack cells.
- Suspended continuations with captured stacks.
- Runtime-provided roots for external resources.
- Globals.

The VM owns GC and root collection; the runtime provides extra roots.
The design assumes a non-moving collector initially, but uses handle indirection so a moving collector can be introduced without breaking the ABI.
External resources must still be explicitly rooted by the runtime, even with a non-moving collector.

Write barriers are required for generational or concurrent collectors.
Lowering must preserve managed write sites (`field.set`, `element.set`, stores through managed references) so the VM can attach barrier logic.
The barrier model is Go-style Dijkstra with shade-on-write.

# External Resources

External resources live in the runtime and are represented as opaque resource ids in `Value`.
The VM treats resources as opaque and relies on runtime-provided roots.

```ds
type ExternalResource = {
    typeId: uint64,
    payloadPtr: *unknown,
    finalizer?: (resource: ExternalResource) => void,
};

type ResourceId = uint64;
const resourceId: ResourceId = 0;
const resource: ExternalResource = resourceTable[resourceId];
```

Finalizers are runtime-owned and run only at safe points.
Finalizers must not re-enter the VM.
Resources are excluded from snapshots unless the runtime explicitly serializes them.

# Snapshotting

Snapshots capture all VM-visible state.
Snapshots are required to implement `Record` and `Replay`.
A snapshot captures:

- Managed heap and raw heap are captured.
- Globals are captured.
- Call stacks and continuation queues are captured.
- Deterministic sources such as PRNG seed and time epoch are captured.

External resources are excluded by default and require runtime-defined hooks.
If external resources are present and the runtime has no hook, snapshotting must fail loudly.
The raw heap is snapshot-safe because raw pointers are index-based, not native addresses.
Any external resource must be reattached explicitly by the runtime after restore.

Deterministic execution requires the runtime to control time, randomness, scheduling, and bindings.
The VM provides snapshot hooks and must be able to replay execution when those sources are fixed.

# VM <-> Native Transitions

Transitions require explicit metadata from lowering:

- **Deopt maps** reconstruct MIR frames from native registers and stack.
- **OSR maps** enter native code from MIR block boundaries.
- **GC stack maps** mark live managed references in native frames.

```ds
type FunctionId = uint32;
type BlockId = uint32;
type InstructionId = uint32;

type DeoptMapId = uint32;
type StackMapId = uint32;
type OsrEntryId = uint32;

type Safepoint = {
    pc: uint64,
    deoptMap: DeoptMapId,
    gcStackMap: StackMapId,
    osrEntry?: OsrEntryId,
};

type DeoptMap = {
    frames: FrameMap[],
};

type FrameMap = {
    function: FunctionId,
    block: BlockId,
    instruction: InstructionId,
    values: ValueLoc[],
    locals: ValueLoc[],
    returnDestination?: ValueLoc,
};

type Register = { kind: 'reg', index: uint16 };
type StackSlot = { kind: 'stack', index: uint32, offset: int32 };
type Constant = { kind: 'const', value: ConstantValue };
type ValueLoc = Register | StackSlot | Constant;
```

## Map Semantics

The metadata follows these rules:
- frame order is outermost to innermost, with the last frame as the active one
- `values` and `locals` are indexed by MIR ids and must cover live values
- GC stack maps mark managed references using MIR type information
- OSR entries are only valid at block boundaries with a known live set
- missing values are materialized as `Void` during reconstruction (?)
- constants use the MIR constant encoding and do not require native storage

The runtime decides when to transition; the VM guarantees reconstruction correctness.
Metadata formats are compiler-version private and must be version-checked by the runtime.
The wire types for these metadata tables live in `destack_vm_abi`.

## Transition Policy

Transition fidelity is policy-controlled and tied to `debugMode`:
- `Vm` runs the interpreter only and does not require native metadata.
- `Deopt` requires full value and local reconstruction at every safepoint.
- `Native` permits minimal metadata (crash-only).

Inline frame encoding is required for `Deopt`.
OSR entries follow `osrMode` and require full live-value materialization.
Safepoints follow `safepointMode`, with optional instruction-budget checks via `safepointInterval`.
Deopt is forbidden across non-replayable operations (FFI, I/O, atomics, runtime callbacks).
Stack maps are exact at all safepoints.

## Tiering and Profiling

The execution model is two-tier:
- **VM** for comptime, deterministic debugging, and fallback execution.
- **Compiled** code for production performance.

Tiering decisions are owned by the runtime.
The VM discovers compiled entrypoints at call sites and OSR boundaries when they exist.

Profiling follows `profilingMode`, and the captured signals are scoped per isolate and consumed by the optimizer:
- Call counts per function and callsite.
- Loop backedge counts.
- Allocation counts and bytes per site.
- Guard failures and deopt reasons.
- Branch direction bias per conditional branch.
- Indirect call target counts for interface dispatch.
- PC sampling for hot instruction ranges when sampling is enabled.

Speculative optimizations follow `speculationMode` and must have guards plus deopt metadata.

## Mixed-tier Execution

The VM supports mixed-tier execution on a per-function basis:
- every function has MIR and can run in the interpreter
- compiled entrypoints are optional and discovered at runtime
- call sites jump to compiled code when available, otherwise interpret
- OSR entries allow hot loops to jump into compiled code
- compiled frames deopt to the VM at safepoints

The runtime controls when compiled entrypoints exist (AOT, JIT, or none).

# Telemetry

The VM exposes:
- Counters for instruction counts, calls, and allocations.
- Sampling for opcode-level time distribution.
- Trace hooks as an event stream for timeline tooling.

The runtime aggregates and renders these signals.
Telemetry libraries should correlate VM signals with runtime traces.

# Determinism

Given the same input values and runtime configuration, VM execution is deterministic.
Time, randomness, and scheduling are provided by the runtime and must be explicitly configured for deterministic simulations.

Determinism is controlled via `determinismMode`:
- `Deterministic` uses a deterministic scheduler and fixed PRNG seeds.
- `Record` uses runtime-provided recording for external I/O.
- `Replay` uses runtime-provided replay for external I/O.

`Record` and `Replay` require external calls to be routed through runtime shims.
Record and Replay enable time-travel debugging and simulation testing in userland libraries.

# Production Requirements

Production-grade execution requires:
- Byte-accurate accounting for managed and raw heaps at every allocation site.
- Explicit stack overflow guards with depth checks plus guard pages where available.
- Validated entry points that reject malformed continuations and incorrect isolate ids.
- Versioned metadata formats for safepoints, deopt maps, and stack maps.
- Deterministic scheduling controls and snapshot safety for all VM-visible state.
- Low-overhead telemetry with stable counter and sampling formats.

# Implementation Notes

The sections below describe the interpreter implementation.
They are not part of the VM ABI and do not constrain the contract above.

The interpreter enforces heap limits by cell count.
The runtime translates byte budgets into cell limits until the VM enforces byte-accurate limits directly.

## Value Representation

Values use a compact 16-byte packed representation (data + tag/width metadata).
Threaded decode builds a per-function value kind table so hot integer/float ops skip tag checks.
NaN-boxing is deferred until width/type information can move into instruction metadata.

Aggregates are heap-allocated today with handles in `Value`.
Small aggregate inlining is planned once the 8-byte representation lands.
External resources use a dedicated tag and store a runtime resource id in the value payload.

## Dispatch

The interpreter uses direct threading for efficient dispatch.
Each instruction handler jumps directly to the next without returning to a central loop.
Threaded decode precomputes handler pointers and compact instruction data.

Common instruction sequences may be fused into super-instructions:
- load field, then load another field
- compare and branch
- load, add constant, store

## Caching

Type-dependent operations may use inline caches for fast repeated access.
First access populates the cache; subsequent accesses hit the fast path.
This is critical for property access and reflection-heavy workloads.
Caches do not rely on hidden classes or runtime shape transitions.
Native layouts are static and caches are keyed by type descriptors and itabs.

# Intrinsics

Intrinsics are primitive operations handled directly by the interpreter.
Representative categories include:

| Category | Examples |
|----------|----------|
| Reflection | `sizeOf`, `alignOf`, `typeOf` |
| Bit manipulation | `clz`, `ctz`, `popcnt`, `byteSwap` |
| Checked arithmetic | `add.overflow`, `sub.overflow` |
| Memory | `memcpy`, `memmove`, `memset` |
| Float math | `sqrt`, `sin`, `cos`, `pow`, `floor` |
| Control | `unreachable`, `abort`, `breakpoint` |

Comptime-only intrinsics (`sizeOf`, `alignOf`, `typeOf`) are evaluated during compilation and replaced with constants.
Native codegen never sees them directly.

Semantically void intrinsics (`volatile.load`, `atomic.*`, `prefetch`) execute but do nothing special in the interpreter.
This preserves comptime compatibility.

# Debug Mode

In debug mode, the VM provides full introspection, including:

- breakpoints at MIR locations
- step, step-over, step-out
- inspect locals, globals, and SSA values
- evaluate expressions in context

# Deoptimization

Native code can transfer execution to the VM at safepoints to enable debugging and better introspection.

## Deopt Sequence

```ds
fn triggerDeopt() {
    nativeState = captureRegistersAndStack();
    safepoint = lookupSafepoint(nativeState.pc);
    interpreterState = reconstructState(nativeState, safepoint);
    vm.continueFrom(interpreterState);
}
```

The interpreter receives:
- which function/block/instruction to resume at
- reconstructed SSA values and locals
- the call stack (caller frames reconstructed from metadata)

Execution continues in the VM with full debugging capabilities.
