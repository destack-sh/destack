# MIR

Machine-level(-ish) IR for Destack for native codegen and VM execution.

## Overview

MIR is Destack's low-level target-aware SSA IR. 
Our MIR is a pretty standard low-level IR with _some_ extras:
1. **SSA with block parameters** (like Cranelift, MLIR, Swift SIL) instead of phi nodes
2. **Value-semantic aggregate operations** (like LLVM, Swift SIL) for constructing and destructuring
3. **Memory-semantic aggregate operations** (like Cranelift) for pointer-based access

Admittedly, compared to some other "MIR-level" IR in related compilers, this MIR is still somewhat high-level, but it's not _as_ high-level as DIR is and it's actually executable efficiently so we'll just call it low-level and move on. 

## Blocks and Values

MIR uses single static assignment form (SSA) with block parameters, quite similar to what MLIR, Cranelift, and Swift's SIL do.
(As a reminder, SSA means that every value is assigned exactly once. That's it.)
Unlike most IRs, our SSA values are explicitly typed at their definition site in MIR text, which we found to be significantly easier to review and even slightly easier to implement.
It's all the same in the main logic though.

### Blocks

Functions introduce named parameters which are passed to the first block, usually `entry`; each block has 0-n parameters (typed SSA values), 0-n instructions, and exactly one terminator.

```mir
entry: // block and value names are free-form
    v0: int32 = 1
    second: int32 = 2
    jump add(second)

add(value: int32):            // value comes from predecessor
    sum: int32 = int.add v0, value // can still use first from dominating block
    return sum
```

When a block has multiple predecessors, each can provide its own arguments:

```mir
entry:
    branch condition, merge(left), merge(right)

merge(value: int32):          // value is left or right depending on which edge
    ...
```

### Values

Values are literally just numbered "slots" in the function's "value table" (`Function.values`), numbered sequentially from 0 through the end of the block (running through all blocks).
(Indeed, in the VM that's how the Interpreter stores (inline) values: one big array of value slots.)

### Terminators

Every block ends with one terminator; this is what makes it a "basic" block.
The terminators themselves are also quite straightforward: control flow can return out of the function, jump to another block, suspend, call with an explicit continuation, or trap:

| Terminator | Description | Example |
|------------|-------------|---------|
| `return` | Returns to the caller's frame with a value. | `return v0` |
| `jump` | Unconditionally jumps to another block. | `jump b1(v0)` |
| `branch` | Conditionally jumps to one of two blocks based on a boolean value. | `branch v0, b1(v1), b2(v2)` |
| `check` | Conditionally jumps to a success or failure block based on a semantic constraint (`bounds`, `null`, `zeroDivisor`, etc.); easier to optimize than `branch` because the guard kind is explicit. | `check bounds.u v0, v1, v2 -> b1, b2` |
| `switch` | Jumps to one of many blocks based on an integer value. | `switch v0, b3, 0 => b1, 1 => b2` |
| `call` (`.indirect`, `.virtual`, `.dynamic`) | Calls and branches to an explicit continuation; the suffix picks static, function-value, virtual, or dynamic-table dispatch. | `call foo(v0): (int32) -> int32 -> okBlock` |
| `tailCall` (`.indirect`, `.virtual`, `.dynamic`) | Same dispatch flavors, but reuses the current frame and never returns to the caller. | `tailCall foo(v0): (int32) -> void` |
| `yield` | Suspends the coroutine, yielding a value and remembering where to resume. | `yield v0 -> resume(v1)` |
| `panic` | Starts unwinding the Worker with an optional readonly managed string payload. | `panic v0` |
| `unwind.resume` | Ends a cleanup block by continuing the unwind to the next cleanup or boundary. | `unwind.resume` |
| `trap` | Terminates unrecoverably without unwinding (no cleanup runs). | `trap.abort` |
| `unreachable` | Asserts that this point is never reached; panics if it is. | `unreachable` |

### Continuations

Calls and yields name their continuation explicitly, and can name a cleanup block after a pipe.
The normal target always comes first and the unwind target second, positional just like `branch` and `check` edges.

```mir
call open(v0): (int32) -> File -> done(v1) | cleanup
yield v0 -> resume(v1) | cleanup
```

## Instructions

Instructions perform "operations" and may produce SSA `Value`s.

| Category | Instructions |
|----------|--------------|
| Constants | `const` |
| Arithmetic | `int.*`, `float.*` |
| Type conversion | `cast.*` |
| Selection | `select` (conditional value without branching) |
| Local variables | `local.get`, `local.set`, `local.address` |
| Globals | `global.address` |
| Functions | `function.address`, `closure.bind`, `closure.environment` |
| Memory | `load`, `store`, `pin`, `unpin`, `drop` |
| Aggregates | `struct`, `tuple`, `array`, `field.get`, `field.set`, `field.address`, `element.get`, `element.set`, `element.address` |
| Vector | `vector.*` (splat, extract, insert, shuffle, select, reduce, compare, convert) |
| Tensor | `tensor.*` (splat, extract, load, store, fill, copy, reshape, broadcast, transpose, cast, view, slice, pad, concat, compare, select, reduce, dot, convolution, gather, scatter, convert) |
| Calls | `call`, `call.virtual`, `call.dynamic`, `call.indirect` |
| Allocation | `new.zeroed`, `new.uninit`, `new.complete`, `new.slice.zeroed`, `new.slice.uninit`, `frame.alloc.*` |
| Intrinsics | `intrinsic.*` |

Canonical MIR uses camelCase for multiword instruction and intrinsic names, and spells checks guard-first: `check int.add.overflow.s left, right -> ok, fail`.

### Pointers and References

MIR uses one `ref` carrier to model a base type plus storage, access, and place, spelled payload-first:

```mir
ref<int32, raw, space(shared)>
ref<int32, raw, readonly, space(gpu)>
```

| Kind | Mutability | Example | Meaning |
| --- | --- | --- | --- |
| managed | mutable or readonly | `ref<Point, managed>`, `ref<Point, managed, readonly>` | managed object reference, logical by default |
| unique | mutable or readonly | `ref<Point, unique>`, `ref<Point, unique, readonly>` | unique typed heap reference |
| borrowed | mutable | `ref<Point, borrowed>` | mutable borrow (`&T`) |
| borrowed | readonly | `ref<Point, borrowed, readonly>` | readonly borrow (`&readonly T`) |
| raw | mutable | `ref<int32, raw>` | raw pointer (mutable) |
| raw | readonly | `ref<int32, raw, readonly>` | raw pointer (readonly) |

## Type Aliases

The MIR text format supports - entirely optional - named type aliases for readability.
Aliases are really just syntactic sugar over concrete layouts.

```mir
type Point {
    x: int32;
    y: int32;
}
```

Aliases are referenced with plain names in type positions; the underlying MIR still stores the concrete type.

## Functions

Functions contain basic blocks forming a CFG.
Imported functions have no body (`entry: None`, empty blocks).

```mir
@cold
function kernel(): void {
entry:
    return
}
```

## Globals

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
String initializers require a managed reference to the builtin String layout.
Mutable globals are module-level state (use sparingly).

## Attributes

Attributes can be attached to functions, globals, type aliases, and struct fields.
The supported forms are `@name`, `@name(value)`, and `@name(key=value, ...)`.
Attribute values support identifiers, integers, floats, strings, and lists.

```mir
@cold
@inline
function vertexMain(): void {
entry:
    return
}

@packed
type Point {
    @offset(0)
    x: int32;

    @offset(4)
    y: int32;
}

@section(".rodata")
global Message: ref<String, managed, readonly>, readonly = "hello"
```

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_mir
cargo test -p destack_test --test optimize

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
