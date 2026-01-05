# Machine

MIR interpreter for comptime execution, debug mode, and deoptimization.
The Machine executes MIR directly without compiling to native code, serving two main roles:
1. **Comptime**: Evaluate `comptime { }` blocks during compilation.
2. **Debugging**: Run programs with full introspection for development (even de-opt from native).

# Interpreter

## Value Representation

Values currently use a compact 16-byte packed representation (data + tag/width metadata).
Threaded decode builds a per-function value kind table so hot integer/float ops can skip tag checks.
NaN-boxing is deferred until we can move width and type information into instruction metadata.

Aggregates are heap-allocated today with handles in `Value`.
Small aggregate inlining is planned once the 8-byte representation lands.

## Dispatch

The interpreter uses direct threading for efficient dispatch.
Each instruction handler jumps directly to the next without returning to a central loop, which eliminates branch misprediction on the dispatch.
Threaded decode precomputes handler pointers and compact instruction data, and this is the only execution path.

Common instruction sequences may be fused into super-instructions:
- Load field, then load another field (nested access)
- Compare and branch (conditionals)
- Load, add constant, store (increment patterns)

## Caching

Type-dependent operations may use inline caches for fast repeated access.
First access populates the cache; subsequent accesses hit the fast path.
This helps a lot with property access and type reflection.

# Memory

The Machine manages three kinds of memory, matching MIR's semantics:

| Kind | Instruction | Lifetime | Use Case |
|------|-------------|----------|----------|
| Managed | `managed.alloc` | GC-tracked | Normal objects, TS semantics |
| Raw | `raw.alloc`/`raw.free` | Manual | Performance-critical, `^T` types |
| Stack | `stack.alloc` | Frame-scoped | Temporaries, small allocations |

The managed heap does garbage collection during interpretation.
The raw heap tracks allocations for leak detection in debug builds.
Stack allocations get freed automatically when the frame exits.

Aggregate access uses value operands for `field.get/set` and `element.get/set`.
Pointer access goes through `field.addr` or `element.addr` with `load` or `store`.

# Intrinsics

Intrinsics are primitive operations handled directly by the interpreter:

| Category | Examples |
|----------|----------|
| Reflection | `sizeOf`, `alignOf`, `typeOf` |
| Bit manipulation | `clz`, `ctz`, `popcnt`, `byteSwap` |
| Checked arithmetic | `add.overflow`, `sub.overflow` |
| Memory | `memcpy`, `memmove`, `memset` |
| Float math | `sqrt`, `sin`, `cos`, `pow`, `floor` |
| Control | `unreachable`, `abort`, `breakpoint` |

**Comptime-only intrinsics** (`sizeOf`, `alignOf`, `typeOf`) get evaluated during compilation and replaced with constants.
Native codegen never sees them (directly).

**Semantically void intrinsics** (`volatile.load`, `atomic.*`, `prefetch`) execute but don't do anything special in the interpreter.
This lets comptime code include patterns that use these operations without breaking.

# External Functions

Functions that can't be interpreted (FFI, system calls) are registered as external handlers.
They get called with marshaled arguments and return marshaled results.

In debug mode, external calls can be wrapped with crash protection.
If an FFI call crashes, the interpreter state is preserved for inspection.

# Debug Mode

In debug mode, the Machine provides full introspection:

**Breakpoints**: Pause execution at specific MIR locations.

**Stepping**: Execute one instruction, step over calls, or step out of functions.

**Inspection**: View call stack, local variables, evaluate expressions in context.

**Watches**: Monitor expressions and pause when values change.

Debug mode uses the same interpreter as comptime.
The only difference is that debug commands can pause and inspect execution.

# Deoptimization

Native code can transfer execution to the Machine at safepoints.
This enables debugging of optimized code by continuing in the interpreter.

## Safepoints

Compiled code includes safepoints where deoptimization can occur:
- Function calls
- Loop back-edges
- Before potentially blocking operations

At each safepoint, metadata describes how to reconstruct interpreter state from native registers and stack.

## Deopt Sequence

When a breakpoint is hit or deopt is requested:

```ds
// pseudocode: deoptimization sequence
fn triggerDeopt() {
    // capture where we are in native code
    nativeState = captureRegistersAndStack()

    // find the safepoint metadata for this location
    safepoint = lookupSafepoint(nativeState.pc)

    // reconstruct interpreter state from native state
    interpreterState = reconstructState(nativeState, safepoint)

    // transfer to interpreter and continue
    machine.continueFrom(interpreterState)
}
```

The interpreter receives:
- Which function/block/instruction to resume at
- Reconstructed SSA values and locals
- The call stack (caller frames are also reconstructed)

After deopt, execution continues in the interpreter with full debugging capabilities.
