# VM

The VM is the MIR execution engine used for comptime evaluation, debugging, deterministic replay, and deopt fallback from native code.
It is fully featured for MIR, but it never performs platform I/O directly.

## Overview
The VM runs MIR deterministically while deferring all external effects to the runtime.
It provides yield and continuation support so that scheduling and replay can be driven externally.
It cooperates with native code through OSR, deopt, and stack maps in a way that mirrors the V8 and JSC split between engine and host.

## Boundaries and Entry Points
All external effects cross the runtime boundary.
The VM never issues syscalls, opens files, or touches network resources.
External resources are represented as opaque values and managed by the runtime.

Entry points are single threaded per isolate and are never reentrant.
The runtime may run many isolates across OS threads, but it serializes entry point calls per isolate.
The runtime uses a single scheduler for both VM and native tasks by treating continuations as opaque runnables.

The runtime and VM divide ownership cleanly.

| State | Owner | Contents | Snapshot |
|-------|-------|----------|----------|
| isolate state | VM | managed heap, raw heap, globals, options | yes |
| execution state | VM | call stack, value stack, local stack | yes |
| continuation state | VM | suspended stacks plus resume point | yes |
| runtime state | runtime | scheduler queues, timers, external resources | runtime-defined |

## Continuations and Yielding
Yield suspends execution and returns an opaque continuation.
Continuations are single shot unless explicitly forked, and the runtime must not inspect or mutate their internals.

All external effects are routed through the runtime.
The VM yields at effect boundaries when required by policy, and the runtime schedules resumes and injects the effect result as the resume value.

## Memory Model
The VM provides managed, raw, and stack allocations.
The managed heap uses precise tracing, and the runtime may contribute extra GC roots for external resources.
Raw allocations are explicit and must be freed manually.
Stack allocations are frame scoped and disappear on return.
The managed heap currently lives inside the VM, but runtime policy and GC coordination are owned by the runtime and will be shared with native execution.