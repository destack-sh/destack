# machine

MIR interpreter for comptime execution.
When you write `comptime { ... }`, the machine is what runs it.

## Overview

The machine crate provides an `Interpreter` that evaluates MIR directly.
It's used by the compiler's `Execute` phase to evaluate compile-time code:
- `comptime` blocks and expressions
- Static parameter arguments (`Container<comptime 42>`)
- Constant folding and propagation
- Compile-time reflection (`sizeOf`, `typeOf`, etc.)

```
Lower → MIR → Verify → Execute (this crate) → Optimize → Generate
                          │
                    comptime results
                    feed back into MIR
```

Similar to Zig's comptime or Rust's const evaluation.
Like Zig, we directly interpret MIR rather than JIT-compiling.

## Architecture

The main construct is the `Interpreter` struct:

```ds
struct Interpreter {
    tree: mir.NodeTree,           // the MIR being executed
    strings: ImmutableStringPool,
    managedHeap: ManagedHeap,     // GC-tracked allocations
    rawHeap: RawHeap,             // manually managed allocations
    globals: GlobalStorage,
    externals: Map<string, ExternalFn>,
    callStack: Frame[],
    statistics: Statistics,
}
```

The interpreter walks through MIR instructions, maintaining:
- Bounded call stack of `Frame`s (one per function invocation)
- SSA value bindings per frame
- Local variable storage per frame
- Heap storage for allocated objects

### Memory Model

Three kinds of memory, matching MIR's allocation instructions:

| Kind | Instruction | Lifetime | Use Case |
|------|-------------|----------|----------|
| Managed | `managed.alloc` | GC-tracked | Normal objects, TS semantics |
| Raw | `raw.alloc`/`raw.free` | Manual | Performance-critical, `^T` types |
| Stack | `stack.alloc` | Frame-scoped | Temporaries, small allocations |

### Values

Runtime values are represented by the `Value` enum:

```ds
newtype Value =
    | Void
    | Bool { value: bool }
    | Int { value: int64, width: uint8 }
    | UInt { value: uint64, width: uint8 }
    | Float32 { value: float32 }
    | Float64 { value: float64 }
    | String { value: string }
    | Char { value: char }
    | ManagedReference { handle: HeapHandle }
    | RawPointer { pointer: RawPointer }
    | StackPointer { pointer: StackPointer }
    | GlobalPointer { id: LocalNodeId<Global> }
    | FunctionPointer { id: LocalNodeId<Function> }
    | Aggregate { values: Value[] }
```

Aggregates (structs, tuples, arrays) are boxed slices of values.
Pointers are handles into the appropriate heap/storage.
Aggregate instructions (`field.get/set`, `element.get/set`) can operate on aggregate values
or on managed/raw/stack pointers to aggregates.

## Execution

The interpreter is a straightforward instruction walker:

1. Start at the entry block of the target function
2. Execute instructions in order, updating SSA bindings
3. When hitting a terminator, jump to the appropriate successor block
4. On `call`, push a new frame and recurse
5. On `return`, pop the frame and continue in the caller

### Intrinsics

Most intrinsics are straightforward to implement:
- `size_of`/`align_of`: look up the type's computed layout
- Bit operations (`clz`, `popcnt`): use Rust's intrinsics
- Float math (`sin`, `sqrt`): use Rust's `f64` methods
- Memory operations: manipulate the heap

Some intrinsics don't make sense in comptime and will abort:
- `volatile.load`/`volatile.store` (no memory-mapped I/O)
- `breakpoint` (no debugger attached)
- Atomics (single-threaded interpreter)

### External Functions

For functions that can't be interpreted (FFI, system calls), we can register external handlers via the `Interpreter`:

```ds
interpreter.registerExternal("print", (args) => {
    console.log(args)
    Ok(Value.Void)
})
```
