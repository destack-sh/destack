# MIR

Machine-level Intermediate Representation for Destack.
This is what gets fed to native codegen (Cranelift) and WASM, and what the comptime interpreter (VM) executes.

## Overview

MIR is the low-level IR in the Destack pipeline.
DIR is high-level, target-independent, and polymorphic; MIR is low-level, target-aware, and monomorphic.
It knows pointer sizes and calling conventions, but doesn't commit to specific registers or instruction encodings yet.

## Compilation Unit

The compilation unit defines the scope for inter module optimization and shared metadata.
Module scope is the default compilation unit for fast builds.
Thin LTO uses package scope and Full LTO uses program scope.
Auto selects Thin LTO at O4 and uses module scope at lower levels.

## Symbol Identity

Function names in MIR are the mangled symbol identity.
The mangling is deterministic and stable within the compilation unit.
Package and program scope analyses use these names to stitch cross module edges.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DIR → MIR → NATIVE                             │
│                                                                             │
│  Stages:  Lower ───► Verify ───► Optimize ───► Generate                     │
│  Output:    MIR    checked MIR   optimized MIR   native/WASM                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

MIR uses SSA with block parameters (instead of phi nodes) like MLIR, Cranelift, and Swift's SIL:

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

## Design Philosophy

MIR is a **hybrid IR** that combines:
1. **SSA with block parameters** (like Cranelift, MLIR, Swift SIL) instead of phi nodes
2. **Value-semantic aggregate operations** (like LLVM, Swift SIL) for constructing and destructuring
3. **Memory-semantic aggregate operations** (like Cranelift) for pointer-based access

This hybrid approach serves Destack's multi-backend architecture and mixed value/reference semantics.

### Why Both Value and Memory Semantics?

MIR has two ways to work with aggregates (structs, tuples, arrays):

| Operation | Style | Use Case |
|-----------|-------|----------|
| `aggregate`, `field.get`, `field.set`, `element.get`, `element.set` | Value | Local, non-aliased data |
| `field.addr`, `element.addr` + `load`/`store` | Memory | Data behind references, address-taking |

**Value operations** treat aggregates as immutable SSA values:
```mir
v0 = aggregate (i32, i32) (v1, v2)   ; construct a tuple
v3 = field.get v0, 0                  ; extract first element (new SSA value)
v4 = field.set v0, 1, v5              ; "update" creates new tuple value
```

**Memory operations** compute addresses for load/store:
```mir
v0 = field.addr v1, 0                 ; get address of field 0
v2 = load v0                          ; load through pointer
store v0, v3                          ; store through pointer
```

### Comparison with Other IRs

| IR | Block Params | Value Aggregates | Memory Aggregates | Notes |
|----|--------------|------------------|-------------------|-------|
| **LLVM** | No (phi nodes) | Yes (`extractvalue`, `insertvalue`) | Yes (`getelementptr`) | Maximum expressiveness |
| **Cranelift** | Yes | No | Yes (stack slots + loads) | Simplicity for fast JIT |
| **Swift SIL** | Yes | Yes (`struct_extract`, `tuple_extract`) | Yes (`struct_element_addr`) | Mixed semantics like Destack |
| **MLIR** | Yes | Dialect-dependent | Dialect-dependent | Extensible framework |
| **Destack MIR** | Yes | Yes | Yes | Matches source semantics |

Cranelift chose memory-only to simplify their implementation—they're optimized for fast compilation as a JIT backend, not maximum optimization potential.
LLVM and Swift SIL use the same hybrid approach as MIR because:
- Value operations enable cleaner dataflow analysis (no aliasing concerns)
- Memory operations are necessary when addresses are taken or data is behind references

### When to Use Which

**Use value operations when:**
- Constructing fresh aggregates (`aggregate`)
- Destructuring local values (`field.get`, `element.get`)
- Transforming values without aliasing (`field.set`, `element.set`)
- The aggregate is an SSA value, not behind a reference

**Use memory operations when:**
- Accessing through a reference (`&T`, `&mut T`, `^T`, `*T`)
- Taking the address of a field or element
- The result needs to be a pointer (for passing to functions, etc.)

### Benefits of the Hybrid Approach

1. **VM execution**: The comptime interpreter can manipulate aggregate values directly without simulating memory.
2. **Optimization**: Value-semantic operations have no aliasing—`aggregate` creates a fresh value, `field.set` produces a new value. SROA (scalar replacement of aggregates) is straightforward.
3. **Backend flexibility**: Native codegen can lower large value aggregates to stack slots while keeping small ones in registers. The IR doesn't force a choice.
4. **Source semantics**: TypeScript++ has both value types (tuples, small structs) and reference types (objects, classes). MIR naturally represents both.
5. **Clear intent**: `field.get` on an SSA value vs `load` through `field.addr` communicates whether we're extracting a copy or accessing shared mutable state.

## Instructions

Instructions produce SSA values and perform operations.
Each instruction defines at most one `Value`.

| Category | Instructions |
|----------|--------------|
| Constants | `const` |
| Arithmetic | `binary`, `unary` |
| Type conversion | `cast` (trunc, extend, bitcast, float↔int, etc.) |
| Selection | `select` (conditional value without branching) |
| Local variables | `local.get`, `local.set` |
| Globals | `global.addr`, `global.const` |
| Memory | `load`, `store`, `drop` |
| Aggregates | `aggregate`, `field.get`, `field.set`, `field.addr`, `element.get`, `element.set`, `element.addr` |
| Calls | `call`, `call.indirect` |
| Allocation | `managed.alloc`, `raw.alloc`, `raw.free`, `stack.alloc` |
| Intrinsics | `intrinsic` |

`field.get/set` and `element.get/set` operate on aggregate values.
To access through pointers, use `field.addr` or `element.addr` and then `load`/`store`.
`Local`s are stack slots for mutable bindings.
SSA values are immutable.
To mutate, allocate a `Local` and use `local.get`/`local.set` (or just use a new value).

`field.addr` and `element.addr` produce a reference to a field or element.
They are helpful for explicit borrowing and aliasing rules.

`drop` invokes the type-specific drop glue for owned values.
Drop glue calls `Symbol.dispose` when the type implements the `Drop` marker.

## Memory Semantics and Metadata

MIR carries precise memory semantics through explicit fields on `Function`.
MIR carries callsite and access metadata through the `NodeTree.call_table` and `NodeTree.memory_table` tables.
Optimizations use it to reason about aliasing, effects, and access sizes (without having to re-derive them).
The memory metadata includes:
- **Function memory effects**: `readnone`, `readonly`, `writeonly`, or `readwrite`, plus a location set indicating which memory regions may be accessed (`arguments`, `heap`, `stack`, `global`, `shared`, `local`, `constant`, `inaccessible`, `io`), an optional address space mask when known, and flags for `argmemonly`, `inaccessibleMemOnly`, and `nosync`.
- **Function behaviors**: `noreturn`, `will_return`, `convergent`, plus `allocates`/`frees` with optional location and address space refinements.
- **Allocator size metadata**: `alloc_size` ties allocator returns to parameter sizes for more precise aliasing and bounds reasoning.
- **Pointer attributes** for parameters and returns: `noalias`, `capture`, `readonly`, `writeonly`, `nonnull`, `noundef`, `dereferenceable`, `dereferenceable_or_null`, `align`, and `returned`.
- `capture` is one of `nocapture`, `return_only`, `store`, or `escape`.
- **Callsite memory metadata**: overrides for memory effects and argument attributes when a call is known to be more precise than the callee signature.
- **Instruction access metadata**: size, alignment, volatile, invariant, non temporal, ordering, address space, alias scopes, and TBAA tags for each memory access.
- **Alias scopes and TBAA tables** describe scoped aliasing and type-based alias analysis relationships.

Instructions with multiple memory accesses, such as `memcpy`, record multiple access descriptors.
Function effects and pointer attributes live on `Function`.
Callsite and per access metadata live in the call and memory tables.
Backends and optimizers query the metadata directly.

### Metadata Structures

This section defines the canonical metadata structures used by MIR.
These structures are stored either on `Function` or in the `NodeTree` metadata tables.

```ds
enum MemoryLocation {
    Arguments,
    Heap,
    Stack,
    Global,
    Shared,
    Local,
    Constant,
    Inaccessible,
    Io,
}

type MemoryLocationSet = MemoryLocation[]

struct MemoryEffect {
    reads: bool,
    writes: bool,
    locations: MemoryLocationSet,
    addressSpaces: AddressSpaceSet | null,
    argmemonly: bool,
    inaccessibleMemOnly: bool,
    nosync: bool,
}

struct AllocSize {
    elementSizeIndex: u32,
    elementCountIndex: u32 | null,
}

struct CallBehavior {
    noreturn: bool,
    willReturn: bool,
    convergent: bool,
    allocates: bool,
    allocLocations: MemoryLocationSet | null,
    allocAddressSpaces: AddressSpaceSet | null,
    frees: bool,
    freeLocations: MemoryLocationSet | null,
    freeAddressSpaces: AddressSpaceSet | null,
}

enum CaptureKind {
    NoCapture,
    ReturnOnly,
    Store,
    Escape,
}

struct PointerAttributes {
    nonnull: bool,
    noalias: bool,
    capture: CaptureKind,
    readonly: bool,
    writeonly: bool,
    noundef: bool,
    dereferenceableBytes: u64 | null,
    dereferenceableOrNullBytes: u64 | null,
    alignment: u32 | null,
    returned: bool,
}

enum ArgumentAccess {
    None,
    Read,
    Write,
    ReadWrite,
}

struct CallArgumentMetadata {
    attributes: PointerAttributes,
    access: ArgumentAccess,
    aliasScopes: AliasScopeId[],
    noaliasScopes: AliasScopeId[],
    tbaaTag: TbaaTagId | null,
}

enum CallDispatchKind {
    Direct,
    Virtual { slotId: u32 },
    Interface { slotId: u32 },
    Indirect,
    Dynamic,
}

struct CallMetadata {
    dispatch: CallDispatchKind,
    receiver: Value | null,
    declaredTarget: LocalNodeId<Function> | null,
    declaringType: LocalNodeId<Type> | null,
    signature: LocalNodeId<Type>,
    memoryEffects: MemoryEffect | null,
    behavior: CallBehavior | null,
    allocSize: AllocSize | null,
    argumentMetadata: CallArgumentMetadata[],
    returnAttributes: PointerAttributes,
}

enum MemoryAccessKind {
    Read,
    Write,
    ReadWrite,
    ReadModifyWrite,
    Fence,
    PrefetchRead,
    PrefetchWrite,
}

enum MemoryAccessTarget {
    Pointer(Value),
    Local(LocalNodeId<Local>),
    Global(LocalNodeId<Global>),
    Unknown,
}

type AddressSpaceSet = AddressSpace[]

struct MemoryAccessMetadata {
    kind: MemoryAccessKind,
    target: MemoryAccessTarget,
    size: u64 | null,
    alignment: u32 | null,
    isVolatile: bool,
    isInvariant: bool,
    isNonTemporal: bool,
    ordering: MemoryOrdering | null,
    addressSpace: AddressSpace | null,
    aliasScopes: AliasScopeId[],
    noaliasScopes: AliasScopeId[],
    tbaaTag: TbaaTagId | null,
}
```

`MemoryOrdering` follows the atomic ordering definitions in the instruction section.
`AddressSpace` follows the reference type address space definitions.
`MemoryLocationSet` summarizes effect regions, while `addressSpaces` optionally refines which address spaces may be accessed.
Per-access address space is recorded in `MemoryAccessMetadata.addressSpace`.

### Metadata Invariants

These invariants keep metadata sound for optimization and codegen.
- Pointer attributes: `readonly` and `writeonly` are mutually exclusive.
- Pointer attributes: `dereferenceableOrNullBytes` is only valid when the pointer can be null.
- Call behavior: `noreturn` implies `willReturn` is false.
- Call behavior: `allocLocations` is only set when `allocates` is true.
- Call behavior: `allocAddressSpaces` is only set when `allocates` is true.
- Call behavior: `freeLocations` is only set when `frees` is true.
- Call behavior: `freeAddressSpaces` is only set when `frees` is true.
- Memory effects: `reads` and `writes` are both false only when `locations` is `NONE`.
- Memory effects: `argmemonly` implies `locations` is `ARGUMENTS` or `NONE`.
- Memory effects: `inaccessibleMemOnly` implies `locations` is `INACCESSIBLE` or `NONE`.
- Memory effects: `nosync` implies the operation does not perform atomic or fence operations.
- Memory access: `ordering` is only set for atomic accesses.
- Memory access: `isInvariant` is only set for read only accesses.
- Memory access: `isVolatile` implies the access cannot be eliminated or reordered.
- Type layout: `fieldOffsets` length matches the field or element count for the type.
- Dispatch tables: `slots` are ordered exactly as the lowering rules define.

### Metadata Update Points

Lowering provides the initial metadata for types, functions, and debug scopes.
Lowering records callsite metadata when dispatch or effects are known.
Optimization passes may refine callsite metadata and memory access descriptors.
Codegen consumes the metadata without mutating it.

### Terminators

Blocks end with a terminator that transfers control:

| Terminator | Description |
|------------|-------------|
| `return` | Return from function |
| `jump` | Unconditional branch |
| `branch` | Conditional branch (if-then-else) |
| `switch` | Multi-way branch on integer |
| `yield` | Suspend coroutine (generators, async) |
| `check` | Checked branch with semantic constraint |
| `unreachable` | UB if reached (traps/panics usually) |

`check` carries a semantic constraint (bounds, null, division, shift, overflow, etc.) and splits control flow into success and failure paths.

## Intrinsics

Intrinsics are primitive operations handled directly by backends.
Inspired by Rust and Zig (especially for SIMD).
They have no function body: each backend implements them specially.

| Category | Examples |
|----------|----------|
| Reflection | `size_of`, `align_of`, `type_of` |
| Bit manipulation | `clz`, `ctz`, `popcnt`, `byte_swap`, `rotate_left` |
| Checked arithmetic | `add.overflow`, `sub.overflow`, `mul.overflow` |
| Unchecked arithmetic | `add.unchecked`, `div.unchecked`, `shl.unchecked` (UB on overflow, division by zero, or shift out of range) |
| Saturating arithmetic | `add.sat`, `sub.sat` |
| Pointer ops | `transmute`, `addrspace.cast`, `ptr_offset_from`, `raw_eq` |
| Memory | `memcpy`, `memmove`, `memset`, `volatile.load` |
| Atomics | `atomic.load`, `atomic.cas`, `atomic.fetch.add`, etc. |
| Float math | `sqrt`, `sin`, `cos`, `pow`, `floor`, etc. |
| GC barriers | `gc.write_barrier`, `gc.read_barrier` |
| Control | `unreachable`, `abort`, `breakpoint` |
| SIMD | `shuffle`, `splat`, `reduce.add`, etc. |

Reflection intrinsics (`size_of`, etc.) are comptime-only—they get evaluated during compilation and replaced with constants.
The VM handles these; native codegen never sees them.

### Integer Arithmetic Semantics

Integer `binary` operations have defined semantics in MIR.
Addition, subtraction, and multiplication wrap in two's complement.
Shift operators mask the shift amount to the integer bit width.
Signed and unsigned division and remainder trap on division by zero.
Signed division and remainder also trap on `min_value / -1`.

When you need unchecked behavior, use the `*.unchecked` intrinsics.
Unchecked intrinsics have undefined behavior on overflow or division by zero, so optimizers may assume they do not occur.
Checked arithmetic can be modeled explicitly with `add.overflow` and related intrinsics.

## Types

MIR types are concrete and fully resolved (no generics, no inference):

```ds
newtype Type =
    | Void
    | Boolean
    | Int { width: uint16, signed: bool }
    | Float { width: uint16 }
    | Reference { kind: ReferenceKind, addressSpace: AddressSpace, mutability: Mutability, pointee: Type, isNullable: bool }
    | Array { element: Type, length: uint64 }
    | Tuple { elements: Type[] }
    | Struct { fields: Field[] }
    | FunctionPointer { parameters: Type[], result: Type }
```

Structs include byte offsets for each field.
Layout is fully computed.
This is what makes MIR "machine-level": no abstract sizes, everything is concrete.

References carry a kind _and_ mutability:
- `managed` for auto-managed references
- `owned` for explicit ownership (`^T`)
- `borrowed` for `&T` and `&mut T`
- `raw` for "unsafe" pointers

Reference syntax spells out the kind and mutability.
Address spaces are optional and appear after the kind.

| Kind | Mutability | Example | Meaning |
| --- | --- | --- | --- |
| managed | none | `ref<managed @T>` | GC-managed reference |
| owned | none | `ref<owned @T>` | owned reference for `^T` |
| borrowed | shared | `ref<borrowed @T>` | shared borrow (`&T`) |
| borrowed | mutable | `ref<borrowed mut @T>` | mutable borrow (`&mut T`) |
| raw | shared | `ref<raw @T>` | raw pointer (immutable) |
| raw | mutable | `ref<raw mut @T>` | raw pointer (mutable) |

Borrowed references are safe aliases verified by the borrow check pass.
Borrows are created by `field.addr`, `element.addr`, and by calls that return borrowed references with lifetimes.
A borrow ends when the reference value is no longer live.
Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
Derived borrows carry provenance so dropping any origin invalidates derived borrows.
Dropping or freeing a value while it is borrowed is always an error.
In strict mode, conflicting borrows and invalidating stores are errors.
In lenient mode, the same situations produce warnings.
Raw references are unsafe pointers with no borrow tracking.
Raw references may be null or dangling and allow pointer arithmetic.
Deref and mutation use explicit `load` and `store` instructions.

Nullable references use `ref?<...>` with the same kind and mutability rules.
Mutability can be encoded for any reference kind, but is only relevant semantically for borrowed and raw references.

Address spaces describe where the reference points:
`generic`, `stack`, `global`, `heap`, `shared`, `local`, `constant`, or a target-specific id.
Use `addrspace(name)` or `addrspace(7)` in the reference syntax:

```mir
ref<raw addrspace(shared) i32>
ref<raw addrspace(7) mut i32>
```

Non generic address spaces are only valid for borrowed and raw references.
`addrspace(constant)` references are always immutable.
`addrspace(generic)` is the default and is omitted in canonical MIR formatting.
Address space changes are explicit and use the `addrspace.cast` intrinsic.

Field names are optional in MIR types and are for readability only:

```mir
type @Point = { x: f32, y: f32 }
```

### Debug Info

MIR preserves enough info to produce high-quality native debug symbols later in codegen.
Lowering records source spans, names, and lexical scope boundaries so codegen can emit DWARF or platform equivalents.
Debug metadata is not required for execution.
Debug metadata is required for precise variable scopes, call stacks, and type names in debuggers.
Debug metadata is stored in `NodeTree.debug_info`.

At a minimum, lowering should provide:
- source spans for every instruction and terminator
- named locals and parameters
- lexical scope boundaries per block and per inlined callsite
- type descriptors for variables and fields

The concrete shape of this metadata belongs in MIR so all backends can consume it consistently.
Cranelift codegen maps this metadata to its own debug facilities and then into DWARF.
The debug metadata table stores scopes, variables, instruction locations, and variable locations.
Scopes are nested and may include inline callsite chains.
Variable locations can reference SSA values, locals, or globals.

### Type Metadata and Dispatch

Type metadata captures layout, lineage, and dispatch structure for nominal types.
Type metadata is stored in `NodeTree.type_table.type_metadata_by_id`.
Layouts store size, alignment, stride, and field offsets in declaration order.
Lineage tracks parent types, interfaces, and sealed or final flags.
Dispatch tables describe vtables and itabs with slot ordering and targets.
Dispatch tables are stored in `NodeTree.type_table.dispatch_tables`.
Type descriptors link types to runtime metadata globals when needed.
Field maps provide name to field lookups for property access specialization.
Struct layouts describe value payloads with no identity semantics.
Class instance types are represented as `ref<managed @Payload>` where `@Payload` is the class field layout.
Dispatch metadata is stored out of line, and polymorphic classes include a vtable pointer in the payload layout when dynamic dispatch remains.
Boxing a value is represented as `managed.alloc` of the payload layout followed by `store` of the value.

### Type Aliases

The MIR text format supports named type aliases for readability.
Aliases are purely syntactic sugar over concrete layouts.

```mir
type @Point = { i32, i32 }
type @PointRef = ref<managed @Point>

function @use_point(v0: ref<managed @Point>) -> ref<managed @Point> {
block0(v0: ref<managed @Point>):
    return v0
}
```

Aliases are referenced with `@Name` in type positions.
The underlying MIR still stores and uses the concrete type.
This makes some documentation and tests much more readable.

## Functions and Globals

Functions contain basic blocks forming a CFG:

```ds
struct Function {
    name: StringId,
    parameters: TypedValue[],
    returnType: LocalNodeId<Type>,
    memoryEffects: MemoryEffect | null,
    callBehavior: CallBehavior | null,
    allocSize: AllocSize | null,
    parameterAttributes: PointerAttributes[],
    returnAttributes: PointerAttributes,
    linkage: Linkage,
    allocation: AllocationMode,  // Any, NoManaged, StackOnly
    coroutine: CoroutineKind | null, // Generator, Async, AsyncGenerator
    locals: LocalNodeId<Local>[],
    blocks: LocalNodeId<Block>[],
    entry: LocalNodeId<Block> | null,
}
```

Imported functions have no body (`entry: None`, empty blocks).
The `allocation` field lets us mark functions as realtime-safe (no GC) or embedded-safe (stack only).

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
    yield v0, block1

block1(v1: i32):    // resumed with value from .next(arg) or resolved promise
    ...
```

The `CoroutineKind` (Generator, Async, AsyncGenerator) tells codegen what wrapper to generate.
Resume arguments appear as normal block arguments, with the resumed value appended after them.
