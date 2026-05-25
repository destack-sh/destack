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

Functions introduce named parameters which are passed to the first block, usually `entry`, each block has 0-n parameters and 0-n instructions with one terminator.
Blocks are the basic control flow units with:
 - a single entry point with a list of parameters (typed SSA values)
 - a list of instructions
 - a single terminator

```mir
entry: // can use any name
    v0: int32 = 1
    second: int32 = 2 // can use any name
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
| `call` | Calls a static function and branches to an explicit continuation. | `call foo(v0): (int32) -> int32 -> okBlock` |
| `call.indirect` | Calls a function value and branches to an explicit continuation. | `call.indirect v1(v0): (int32) -> int32 -> okBlock` |
| `call.class` | Dispatches a class method and branches to an explicit continuation. | `call.class receiver, TypeName, 3(v0): (ref<TypeName, managed, readonly>) -> int32 -> okBlock` |
| `call.interface` | Dispatches through an interface table and branches to an explicit continuation. | `call.interface receiver, InterfaceName, 3(v0): (any<InterfaceName>) -> int32 -> okBlock` |
| `tailCall` | Calls a static function and reuses the current frame, never returning to the caller. | `tailCall foo(v0): (int32) -> void` |
| `tailCall.indirect` | Tail-calls through a function value, reusing the current frame. | `tailCall.indirect v1(v0): (int32) -> void` |
| `tailCall.class` | Tail-calls a class method, reusing the current frame. | `tailCall.class receiver, TypeName, 3(v0): (ref<TypeName, managed, readonly>) -> void` |
| `tailCall.interface` | Tail-calls through an interface table, reusing the current frame. | `tailCall.interface receiver, InterfaceName, 3(v0): (any<InterfaceName>) -> void` |
| `yield` | Suspends the coroutine, returning a value and remembering where to resume in a "resume block". | `yield v0, resume(v1)` |
| `trap` | Terminates the program unrecoverably; trap kind is `trap.abort` or `trap.panic`. `trap.panic` carries a non-null readonly managed string payload. | `trap.panic v0` |
| `unreachable` | Asserts that this point is never reached; traps with a panic if it is. | `unreachable` |

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
| Functions | `function.address`, `callable.bind`, `callable.environment` |
| Memory | `load`, `store`, `raw.free`, `pin`, `unpin`, `drop` |
| Aggregates | `struct`, `tuple`, `array`, `field.get`, `field.set`, `field.address`, `element.get`, `element.set`, `element.address` |
| Vector | `vector.*` (splat, extract, insert, shuffle, select, reduce, compare, convert) |
| Tensor | `tensor.*` (splat, extract, load, store, fill, copy, reshape, broadcast, transpose, cast, view, slice, pad, concat, compare, select, reduce, dot, convolution, gather, scatter, convert) |
| Calls | `call`, `call.class`, `call.interface`, `call.indirect` |
| Allocation | `new`, `new.slice`, `raw.alloc`, `raw.free`, `stack.alloc` |
| Intrinsics | `intrinsic.*` |

Canonical MIR formatting uses camelCase for multiword instruction and intrinsic names.

`check` carries a semantic constraint (`bounds`, `null`, `zeroDivisor`, `shiftRange`, `overflow`, `dynamicType`, `receiverType`, `interfaceConformance`, etc.) and splits control flow into success and failure paths.
Canonical MIR spells checks guard-first: `check int.add.overflow.s left, right -> ok, fail`.

### Pointers and References

References are MIR carriers with explicit storage and place qualifiers.
Pointer-sized integer types are modeled explicitly.
```mir
ref<int32, raw, space(shared)>
ref<int32, raw, readonly, space(gpu)>
```

In general, MIR uses `ref` to model one base type plus storage, access, and place:
- `managed` for runtime managed object references
- `unique` for unique typed heap references used by `Box`, arrays, and similar storage wrappers
- `borrowed` for `&T` and `&readonly T`
- `raw` for "unsafe" physical pointers

Reference syntax is payload-first and spells out qualifiers after the payload type.

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

function usePoint(point: ref<Point, managed>): ref<Point, managed> {
entry(point: ref<Point, managed>):
    return point
}
```

Aliases are referenced with plain names in type positions.
The underlying MIR still stores and uses the concrete type.

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
