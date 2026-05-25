# VM

The VM is the MIR execution engine used for comptime evaluation, debugging, analysis, deterministic replay, and deopt fallback from native code.
It is fully featured for MIR _logic_, and it performs "external" work only via explicit platform bindings.
So, the VM by itself is just a pure resumable computation engine with serializable state, which is quite nice.

## Overview

The VM runs MIR deterministically while deferring all external effects to the runtime (as above).
It provides yield and continuation support so that scheduling and replay can be driven externally.
It cooperates with native code through OSR, deopt, and stack maps in a way that mirrors the V8 and JSC split between engine and host.

## Execution

All external effects cross the runtime boundary.
The VM never issues syscalls, opens files, or touches network resources.
External resources are represented as opaque values and managed by the runtime.

Entry points are single threaded per isolate and cannot be reentrant (for obvious reasons).
The runtime may run many isolates across OS threads, but it serializes entry point calls per isolate.
The runtime is responsible for driving the VM execution beyond single execution "ticks".

Yielding suspends execution and returns an opaque continuation to be managed by the runtime.
Continuations are "single shot" (i.e., they cannot be resumed multiple times) unless explicitly forked.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_vm
cargo test -p destack_test --test optimize

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
