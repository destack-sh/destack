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
entry:
    first: int32 = 1
    second: int32 = 2
    jump add(second)

add(value: int32):            // value comes from predecessor
    sum: int32 = int.add first, value // can still use first from dominating block
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
The terminators themselves are also quite straightforward: essentially, control flow can either return out of the function, jump to another block, suspend, or abruptly exit by throwing:

| Terminator | Description | Example |
|------------|-------------|---------|
| `return` | Returns to the caller's frame with a value. | `return v0` |
| `jump` | Unconditionally jumps to another block. | `jump b1(v0)` |
| `branch` | Conditionally jumps to one of two blocks based on a boolean value. | `branch v0, b1(v1), b2(v2)` |
| `check` | Conditionally jumps to a success or failure block based on a semantic constraint (`bounds`, `null`, `zeroDivisor`, etc.); easier to optimize than `branch` because the guard kind is explicit. | `check bounds.u v0, v1, v2 -> b1, b2` |
| `switch` | Jumps to one of many blocks based on an integer value. | `switch v0, [0: b1, 1: b2], b3` |
| `invoke` | Calls a static function that may unwind; branches to explicit success and exception successors. | `invoke foo(v0): (int32) -> int32 -> okBlock, catch errBlock` |
| `invoke.indirect` | Calls a function value that may unwind; branches to explicit success and exception successors. | `invoke.indirect fn(v0): (int32) -> int32 -> okBlock, catch errBlock` |
| `invoke.virtual` | Dispatches a virtual method that may unwind; branches to explicit success and exception successors. | `invoke.virtual receiver, TypeName, 3(v0): (ref<TypeName, managed, readonly>) -> int32 -> okBlock, catch errBlock` |
| `invoke.interface` | Dispatches an interface method that may unwind; branches to explicit success and exception successors. | `invoke.interface receiver, InterfaceName, 3(v0): (InterfaceName) -> int32 -> okBlock, catch errBlock` |
| `tailCall` | Calls a static function and reuses the current frame, never returning to the caller. | `tailCall foo(v0): (int32) -> void` |
| `tailCall.indirect` | Tail-calls through a function value, reusing the current frame. | `tailCall.indirect fn(v0): (int32) -> void` |
| `tailCall.virtual` | Tail-calls a virtual method, reusing the current frame. | `tailCall.virtual receiver, TypeName, 3(v0): (ref<TypeName, managed, readonly>) -> void` |
| `tailCall.interface` | Tail-calls an interface method, reusing the current frame. | `tailCall.interface receiver, InterfaceName, 3(v0): (InterfaceName) -> void` |
| `yield` | Suspends the coroutine, returning a value and remembering where to resume in a "resume block". | `yield v0, resume` |
| `throw` | Exits abruptly through the exception path, carrying a managed exception object. | `throw v0` |
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
| Globals | `global.address`, `global.const` |
| Functions | `function.address`, `function.bind`, `function.environment` |
| Memory | `load`, `store`, `raw.drop`, `stack.drop` |
| Aggregates | `struct`, `tuple`, `array`, `field.get`, `field.set`, `field.address`, `element.get`, `element.set`, `element.address` |
| Vector | `vector.*` (splat, extract, insert, shuffle, select, reduce, compare, convert) |
| Tensor | `tensor.*` (load, store, fill, copy, reshape, broadcast, transpose, cast, view, slice, pad, concat, compare, select, reduce, dot, convolution, gather, scatter, convert) |
| Calls | `call`, `call.virtual`, `call.interface`, `call.indirect` |
| Allocation | `managed.alloc`, `managed.allocArray`, `raw.alloc`, `raw.free`, `stack.alloc` |
| Intrinsics | `intrinsic.*` |

Canonical MIR formatting uses camelCase for multiword instruction and intrinsic names.

### Terminators

Blocks end with a terminator that transfers control:

| Terminator | Description |
|------------|-------------|
| `return` | Return from function |
| `jump` | Unconditional branch |
| `branch` | Conditional branch (if-then-else) |
| `switch` | Multi-way branch on integer |
| `yield` | Suspend coroutine (generators, async) |
| `invoke*` terminators | Potentially-unwinding call with explicit success and exception successors |
| `tailCall*` terminators | Non-returning call that reuses the current frame |
| `throw` | Abrupt exceptional exit with a managed exception object |
| `trap` | Unrecoverable runtime termination (`abort`, `panic`) |
| `check` | Checked branch with semantic constraint |
| `unreachable` | UB if reached (traps/panics somehow) |

`check` carries a semantic constraint (`bounds`, `null`, `zeroDivisor`, `shiftRange`, `overflow`, `dynamicType`, `receiverType`, `interfaceConformance`, etc.) and splits control flow into success and failure paths.
Canonical MIR spells checks guard-first: `check int.add.overflow.s left, right -> ok, fail`.

## Intrinsics

Intrinsics are primitive operations handled directly by backends.
They have no function body; hosts implement intrinsics however they like.

| Category | Examples |
|----------|----------|
| Reflection | `sizeOf`, `alignOf`, `typeOf` |
| Bit manipulation | `leadingZeroCount`, `trailingZeroCount`, `populationCount`, `byteSwap`, `bitReverse`, `rotateLeft`, `rotateRight` |
| Checked arithmetic | `add.overflow`, `sub.overflow`, `mul.overflow` |
| Unchecked arithmetic | `add.unchecked`, `sub.unchecked`, `mul.unchecked`, `div.unchecked`, `rem.unchecked`, `shl.unchecked`, `shr.unchecked` |
| Saturating arithmetic | `add.sat`, `sub.sat` |
| Pointer ops | `transmute`, `addressSpace.cast`, `ptrOffsetFrom`, `rawEq` |
| Memory | `memcpy`, `memmove`, `memset`, `memcmp`, `prefetch.read`, `prefetch.write` |
| Float math | `sqrt`, `abs`, `fma`, `copySign`, `min`, `max`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `exp2`, `log`, `log2`, `log10`, `pow`, `floor`, `ceil`, `trunc`, `round` |
| GC barriers | `writeBarrier` |
| Control | `breakpoint`, `returnAddress`, `frameAddress` |
| Branch hints | `expect` |
| Optimization | `blackBox` |

### Pointers

Pointer sized integer types are modeled explicitly.
`isize` is a signed integer with the target pointer width.
`usize` is an unsigned integer with the target pointer width.
Their concrete widths are resolved from module data layout metadata.

### Function Values

Bare code pointers use `fn(...) -> ...`.
Callable closure values use `closure(...) -> ...`.
The signature operand is always a bare function pointer type, and the environment operand carries the captured context reference.
Canonical MIR treats function values as a dedicated two field aggregate rather than an anonymous struct convention.

### References

References carry a kind and reference-level mutability:
- `managed` for runtime managed object references
- `owned` for explicit ownership (`^T`)
- `borrowed` for `&T` and `&readonly T`
- `raw` for "unsafe" physical pointers

Reference syntax is payload-first and spells out qualifiers after the payload type.
Address spaces are optional and appear last.

| Kind | Mutability | Example | Meaning |
| --- | --- | --- | --- |
| managed | mutable or readonly | `ref<Point, managed>`, `ref<Point, managed, readonly>` | managed object reference, logical by default |
| owned | mutable or readonly | `ref<Point, owned>`, `ref<Point, owned, readonly>` | owned handle for `^T` |
| borrowed | mutable | `ref<Point, borrowed>` | mutable borrow (`&T`) |
| borrowed | readonly | `ref<Point, borrowed, readonly>` | readonly borrow (`&readonly T`) |
| raw | mutable | `ref<int32, raw>` | raw pointer (mutable) |
| raw | readonly | `ref<int32, raw, readonly>` | raw pointer (readonly) |

Address spaces describe where the reference points:
`generic`, `stack`, `global`, `shared`, `local`, `constant`, or a target-specific id.
Use `addressSpace(name)` or `addressSpace(7)` in the reference syntax:

```mir
ref<int32, raw, addressSpace(shared)>
ref<int32, raw, readonly, addressSpace(7)>
```

`managed` references always use `addressSpace(generic)`.
`borrowed`, `raw`, and `owned` references may use non generic address spaces when the target and allocator semantics define them.
`addressSpace(constant)` references are always immutable.
`owned` references cannot use `addressSpace(constant)`.
`addressSpace(generic)` is the default and is omitted in canonical MIR formatting.
Address space changes are explicit and use the `addressSpace.cast` intrinsic.

Binding immutability and effect-level readonly facts are separate from reference-level readonly.

Local declarations use the same payload-first qualifier shape and keep an explicit `local` introducer:

```mir
local temp: int32, owned
local valuesLocal: int32[4], owned, readonly
local borrowSlot: ref<int32, borrowed>, borrowed
```

Field names are optional in MIR types and are for readability only:

```mir
type Point {
    x: float32;
    y: float32;
}
```

### Type Aliases

The MIR text format supports named type aliases for readability.
Aliases are purely syntactic sugar over concrete layouts.

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
This makes some documentation and tests much more readable.

## Functions

Functions contain basic blocks forming a CFG.
Imported functions have no body (`entry: None`, empty blocks).

```mir
@executionModel(kernel)
@workgroupSize(8, 1, 1)
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

### Coroutines

Generators and async functions are lowered to state machines in DIR before reaching MIR.
MIR just sees the `Yield` terminator and knows the function is a coroutine:

```mir
entry:
    next: closure() -> int32 = call computeNext(): () -> closure() -> int32
    yield next, resume

resume(value: int32):    // resumed with value from .next(arg) or resolved promise
    ...
```

The `SuspensionKind` (Generator, Async, AsyncGenerator) tells codegen what wrapper to generate.
Resume arguments appear as normal block arguments, with the resumed value appended after them.


## Attributes

Attributes can be attached to functions, globals, type aliases, and struct fields.
They use TS++-style decorator syntax and support bare, value, and key-value forms.
Attributes always apply to the next item or field in the text stream.
The supported forms are `@name`, `@name(value)`, and `@name(key=value, ...)`.
Attribute values support identifiers, integers, floats, strings, and lists.
Canonical MIR formatting uses camelCase decorator names and keys.

```mir
@executionModel(graphics)
@executionStage(vertex)
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

`const` is still accepted by the parser for compatibility.
Canonical MIR formatting always emits `readonly`.

### Linkage

```ds
newtype Linkage =
    | Local   // defined here, not visible outside (private)
    | Export  // defined here, visible outside (public)
    | Import  // declared here, defined elsewhere
```

Exported symbols get mangled names for linking.
Imported symbols reference external definitions (FFI, other modules, runtime).

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_mir
cargo test -p destack_test --test optimize
just language/test-emit

# clean gate
just language/quick

# exhaustive gate
just language/full
```
