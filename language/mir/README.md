# mir

Machine-level Intermediate Representation for Destack.
This is what gets fed to native codegen (Cranelift) and WASM, and what the comptime interpreter executes.

## Overview

MIR is the low-level IR in the Destack pipeline.
DIR (Destack IR) is high-level, target-independent, and polymorphic; MIR is low-level, target-aware, and monomorphic.

```
DIR (semantic, polymorphic)  →  MIR (machine, monomorphic)  →  native/WASM
         │                            │
       Lower                     Generate
```

By the time code reaches MIR:
- All generics are ready to be monomorphized (each `Container<int32>` is its own concrete `Instance`)
- All types have known sizes and layouts
- All control flow is explicit (basic blocks with terminators)
- All values are SSA (Static Single Assignment)

MIR is target-independent but target-aware: it knows pointer sizes and calling conventions, but doesn't commit to specific registers or instruction encodings (yet).

### SSA with Block Parameters

MIR uses SSA (Static Single Assignment) with block parameters instead of phi nodes.
This is the same approach as MLIR, Cranelift, and Swift's SIL.

```mir
block0:
    v0 = const.i32 1
    v1 = const.i32 2
    jump block1(v0)

block1(v2: i32):          // v2 comes from predecessor
    v3 = iadd v1, v2      // can still use v1 from dominating block
    return v3
```

When a block has multiple predecessors, each provides arguments:

```mir
block0:
    branch v0, block2(v1), block2(v2)

block2(v3: i32):          // v3 is v1 or v2 depending on which edge
    ...
```

## Instructions

Instructions produce SSA values and perform operations.
Each instruction defines at most one `Value`.

| Category | Instructions |
|----------|--------------|
| Constants | `const` |
| Arithmetic | `binary`, `unary` |
| Type conversion | `cast` (trunc, extend, bitcast, float↔int, etc.) |
| Local variables | `local.get`, `local.set` |
| Globals | `global.addr`, `global.const` |
| Memory | `load`, `store` |
| Aggregates | `field.get`, `field.set`, `element.get`, `element.set` |
| Calls | `call`, `call.indirect` |
| Allocation | `managed.alloc`, `raw.alloc`, `raw.free`, `stack.alloc` |
| Intrinsics | `intrinsic` |

`Local`s are stack slots for mutable bindings.
SSA values are immutable.
To mutate, allocate a `Local` and use `local.get`/`local.set` (or just use a new value).

### Terminators

Blocks end with a terminator that transfers control:

| Terminator | Description |
|------------|-------------|
| `return` | Return from function |
| `jump` | Unconditional branch |
| `branch` | Conditional branch (if-then-else) |
| `switch` | Multi-way branch on integer |
| `yield` | Suspend coroutine (generators, async) |
| `unreachable` | UB if reached (traps/panics usually) |

## Intrinsics

Intrinsics are primitive operations handled directly by backends.
Inspired by Rust and Zig (especially for SIMD).
They have no function body: each backend implements them specially.

| Category | Examples |
|----------|----------|
| Reflection | `size_of`, `align_of`, `type_of` |
| Bit manipulation | `clz`, `ctz`, `popcnt`, `byte_swap`, `rotate_left` |
| Checked arithmetic | `add.overflow`, `sub.overflow`, `mul.overflow` |
| Unchecked arithmetic | `add.unchecked`, `div.unchecked` (UB on overflow) |
| Saturating arithmetic | `add.sat`, `sub.sat` |
| Memory | `memcpy`, `memmove`, `memset`, `volatile.load` |
| Atomics | `atomic.load`, `atomic.cas`, `atomic.fetch.add`, etc. |
| Float math | `sqrt`, `sin`, `cos`, `pow`, `floor`, etc. |
| GC barriers | `gc.write_barrier`, `gc.read_barrier` |
| Control | `unreachable`, `abort`, `breakpoint` |
| SIMD | `shuffle`, `splat`, `reduce.add`, etc. |

Reflection intrinsics (`size_of`, etc.) are comptime-only: they get evaluated during compilation and replaced with constants.
The interpreter handles these; native codegen never sees them.

## Types

MIR types are concrete and fully resolved (no generics, no inference):

```ds
newtype Type =
    | Void
    | Boolean
    | Int { width: uint16, signed: bool }
    | Float { width: uint16 }
    | RawPointer { pointee: Type }
    | ManagedReference { pointee: Type, isNullable: bool }
    | Array { element: Type, length: uint64 }
    | Tuple { elements: Type[] }
    | Struct { fields: Field[] }
    | FunctionPointer { parameters: Type[], result: Type }
```

Structs include byte offsets for each field.
Layout is fully computed.
This is what makes MIR "machine-level": no abstract sizes, everything is concrete.

## Functions and Globals

Functions contain basic blocks forming a CFG:

```ds
struct Function {
    name: StringId,
    parameters: TypedValue[],
    returnType: LocalNodeId<Type>,
    linkage: Linkage,
    allocation: AllocationMode,  // Any, NoManaged, StackOnly
    coroutine: CoroutineKind | null, // Generator, Async, AsyncGenerator
    locals: LocalNodeId<Local>[],
    blocks: LocalNodeId<Block>[],
    entry: LocalNodeId<Block> | null,
}
```

Imported functions have no body (`entry: None`, empty blocks).
The `allocation` lets you mark functions as realtime-safe (no GC) or embedded-safe (stack only).

### Linkage

```ds
newtype Linkage =
    | Local   // defined here, not visible outside (private)
    | Export  // defined here, visible outside (public)
    | Import  // declared here, defined elsewhere
```

Exported symbols get mangled names for linking.
Imported symbols reference external definitions (FFI, other modules, runtime).

### Globals

Module-level data with optional mutability:

```ds
struct Global {
    name: StringId,
    ty: LocalNodeId<Type>,
    mutability: Mutability,
    linkage: Linkage,
    initializer: GlobalInitializer | null,
}
```

Immutable globals are constants (string literals, lookup tables).
Mutable globals are module-level state (use sparingly).

## Coroutines

Generators and async functions are lowered to state machines in DIR before reaching MIR.
MIR just sees the `Yield` terminator and knows the function is a coroutine:

```mir
block0:
    v0 = call @compute_next()
    yield v0, resume: block1

block1(v1: i32):    // resumed with value from .next(arg) or resolved promise
    ...
```

The `CoroutineKind` (Generator, Async, AsyncGenerator) tells codegen what wrapper to generate.