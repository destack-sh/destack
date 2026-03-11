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
(As a reminder, SSA means that every `vn` value is assigned exactly once. That's it.)
Unlike most IRs, our SSA values are explicitly typed at their definition site in MIR text, which we found to be significantly easier to review and even slightly easier to implement.
It's all the same in the main logic though.

### Blocks

Functions introduce `v0`-`vn` parameters which are passed to the first block, `block0`, each block has 0-n parameters and 0-n instructions with one terminator.
Blocks are the basic control flow units with:
 - a single entry point with a list of parameters (typed SSA values)
 - a list of instructions
 - a single terminator

```mir
block0:
    v0: i32 = iconst 1
    v1: i32 = iconst 2
    jump block1(v0)

block1(v2: i32):          // v2 comes from predecessor
    v3: i32 = iadd v1, v2 // can still use v1 from dominating block
    return v3
```

When a block has multiple predecessors, each can provide its own arguments:

```mir
block0:
    branch v0, block2(v1), block2(v2)

block2(v3: i32):          // v3 is v1 or v2 depending on which edge
    ...
```

### Values

Values are literally just numbered "slots" in the function's "value table" (`Function.values`), numbered sequentially from 0 through the end of the block (running through all blocks).

### Terminators

Every block ends with a single terminator; this is what makes it a "basic" block.
The terminators themselves are also quite straightforward: essentially, control flow can either return out of the function, jump to another block, suspend, or abruptly exit by throwing:
 - `branch`, `check`, `switch` are all just different ways to conditionally jump to another block, with `check` being the most optimisable because it explicitly encodes the condition (which we can later optimize out).
 - `return` jumps back to the caller's frame, while `tailcall` (and `tailcall.indirect`, `tailcall.virtual`, `tailcall.interface`) supersede the _current_ frame.
 - potentially-unwinding calls are also terminators: `call`, `call.indirect`, `call.virtual`, and `call.interface` branch to explicit `normal` and `unwind` successors.
 - `yield` is a special `return` that remembers some values for resuming execution later in a "resume block".
 - `throw` abruptly exits through the exception path and carries a managed exception object.
 - `trap` is unrecoverable runtime termination, with `abort` and `panic` as the current trap kinds.
 - `trap panic` canonically carries a non null readonly managed string payload, while the verifier requires a non null readonly managed reference payload and leaves the runtime string convention to Lower and the runtime.
 - `unreachable` is just a way of signaling "trust me, I can't prove it, but we'll never get here"

## Instructions

Instructions produce SSA values and perform operations.
Each instruction defines at most one `Value`.

| Category | Instructions |
|----------|--------------|
| Constants | `iconst` |
| Arithmetic | `binary`, `unary` |
| Type conversion | `cast` (trunc, extend, bitcast, float↔int, etc.) |
| Selection | `select` (conditional value without branching) |
| Local variables | `local.get`, `local.set`, `local.addr` |
| Globals | `global.addr`, `global.const` |
| Functions | `function.addr`, `function.env` |
| Memory | `load`, `store`, `raw.drop`, `stack.drop` |
| Aggregates | `struct`, `tuple`, `array`, `field.get`, `field.set`, `field.addr`, `element.get`, `element.set`, `element.addr` |
| Vector | `vector.*` (splat, extract, insert, shuffle, select, reduce, compare, convert) |
| Tensor | `tensor.*` (load, store, fill, copy, reshape, broadcast, transpose, cast, view, slice, pad, concat, compare, select, reduce, dot, convolution, gather, scatter, convert) |
| Calls | `call`, `call.virtual`, `call.interface`, `call.indirect` |
| Allocation | `managed.alloc`, `managed.alloc_array`, `raw.alloc`, `raw.free`, `stack.alloc` |
| Intrinsics | `intrinsic` |

### Terminators

Blocks end with a terminator that transfers control:

| Terminator | Description |
|------------|-------------|
| `return` | Return from function |
| `jump` | Unconditional branch |
| `branch` | Conditional branch (if-then-else) |
| `switch` | Multi-way branch on integer |
| `yield` | Suspend coroutine (generators, async) |
| `call*` terminators | Potentially-unwinding call with explicit `normal` and `unwind` successors |
| `throw` | Abrupt exceptional exit with a managed exception object |
| `trap` | Unrecoverable runtime termination (`abort`, `panic`) |
| `check` | Checked branch with semantic constraint |
| `unreachable` | UB if reached (traps/panics somehow) |

`check` carries a semantic constraint (`bounds`, `null`, `div_zero`, `shift`, `overflow`, `type`, `receiver_type`, `implements`, etc.) and splits control flow into success and failure paths.

### Memory

`field.get/set` and `element.get/set` operate on aggregate values.
To access through pointers, use `field.addr` or `element.addr` and then `load` or `store`.
`Local`s are stack slots for mutable bindings.
`local.addr` produces a reference to a local slot.
SSA values are immutable.
To mutate, allocate a `Local` and use `local.get` or `local.set` (or just use a new value).

`field.addr` and `element.addr` produce a reference to a field or element.
Pointer-producing instructions (`local.addr`, `global.addr`, `function.env`, `managed.alloc`, `raw.alloc`, `stack.alloc`, `field.addr`, `element.addr`) define typed SSA values.
`global.addr` returns `ref<raw addrspace(global) T>` and `stack.alloc` returns `ref<raw addrspace(stack) T>`.
`load` defines a typed SSA value for the loaded result.
`cast` includes an explicit target type argument.
`call` instructions include a signature type argument for dispatch and verification.
Typed destinations are required for Core MIR and enable precise alias and ownership analysis.

`raw.drop` ends the lifetime of an owned value and deallocates owned heap storage.
`stack.drop` ends the lifetime of an owned stack value (without explicit deallocation).
These instructions model ownership lifetime ends, not `using` protocol disposal.
`using` lowering is handled separately through disposable protocol calls.
Source language ownership and `using` rules are specified in [language/SPECIFICATION.md](../SPECIFICATION.md).
In Canonical MIR, aggregate addressing should prefer `field.addr` and `element.addr` over raw pointer arithmetic.
Raw pointer arithmetic and pointer reinterpretation should be confined to explicit unsafe boundaries and Lowered MIR legalization.

## Memory Semantics and Metadata

MIR carries precise memory semantics through explicit fields on `Function`.
MIR carries callsite and access metadata through inline `CallEffects` on call instructions and the `NodeTree.memory_table`.
Atomic operations and barriers carry explicit execution scope, memory scope, and memory semantics for GPU and parallel targets.
Optimizations use it to reason about aliasing, effects, and access sizes (without having to re-derive them).
The memory metadata includes:
- **Function memory effects**: `readnone`, `readonly`, `writeonly`, or `readwrite`, plus a region set indicating which semantic memory regions may be accessed (`managed_heap`, `immortal_heap`, `raw_heap`, `stack`, `global`, `shared`, `local`, `constant`, `io`), an optional address space mask when known, and flags for `argmemonly`, `inaccessibleMemOnly`, and `nosync`.
- **Function behaviors**: `repeatability` (`pure`, `repeatable`, `non_repeatable`), `unwind_behavior` (`cannot_unwind`, `may_unwind`), `may_suspend`, `noreturn`, `will_return`, `convergent`, plus `allocates`/`frees` with optional location and address space refinements.
- **Allocator size metadata**: `alloc_size` ties allocator returns to parameter sizes for more precise aliasing and bounds reasoning.
- **Pointer attributes** for parameters and returns: `noalias`, `capture`, `readonly`, `writeonly`, `nonnull`, `noundef`, `dereferenceable`, `dereferenceable_or_null`, `align`, and `returned`.
- `capture` is one of `nocapture`, `return_only`, `store`, or `escape`.
- **Callsite memory metadata**: overrides for memory effects and argument attributes when a call is known to be more precise than the callee signature.
- **Instruction access metadata**: size, alignment, volatile, invariant, non temporal, ordering, address space, alias scopes, and TBAA tags for each memory access.
- **Alias scopes and TBAA tables** describe scoped aliasing and type-based alias analysis relationships.

Instructions with multiple memory accesses, such as `memcpy`, record multiple access descriptors.
Function effects and pointer attributes live on `Function`.
Callsite effects live on call instructions.
Per-access metadata live in the memory table.
Backends and optimizers query the metadata directly.
Call effects remain optional and refine effects, dispatch, and profiling data.
Destack does not have surface `noexcept` syntax.
Instead, unwind behavior is inferred into MIR function and call metadata so ordinary non-throwing calls stay cheap even when exceptions are enabled elsewhere.
Calls that may unwind leave the block through exceptional call terminators with explicit `normal` and `unwind` continuations.

### Layout metadata

Layout tables live in MIR metadata, not in the MIR text format.
Module data layout metadata also lives in MIR metadata (`NodeTree.data_layout`), including native pointer size and managed reference representation.
The parser option for pointer size initializes this module metadata and is not the canonical source of truth.
The layout table stores concrete size, alignment, and field offsets for aggregate types.
The layout table is the single source of truth for physical layout across optimizer, VM, and codegen.
Union metadata describes logical union semantics, while the layout table describes physical offsets.

## Intrinsics

Intrinsics are primitive operations handled directly by backends.
They have no function body; hosts implement intrinsics however they like.
Canonical MIR keeps only one branch hint form: `expect(cond, expected)`.
Atomics and barriers are first class MIR instructions, not intrinsics.
Target specific accelerator and backend operations do not belong in canonical MIR intrinsics.
Canonical MIR also keeps the intrinsic set intentionally small:
control flow lives in terminators, concurrency lives in dedicated instructions, and target specific operations belong in lowered target layers rather than the core MIR.

| Category | Examples |
|----------|----------|
| Reflection | `size_of`, `align_of`, `type_of` |
| Bit manipulation | `clz`, `ctz`, `popcnt`, `byte_swap`, `bit_reverse`, `rotate_left`, `rotate_right` |
| Checked arithmetic | `add.overflow`, `sub.overflow`, `mul.overflow` |
| Unchecked arithmetic | `add.unchecked`, `sub.unchecked`, `mul.unchecked`, `div.unchecked`, `rem.unchecked`, `shl.unchecked`, `shr.unchecked` |
| Saturating arithmetic | `add.sat`, `sub.sat` |
| Pointer ops | `transmute`, `addrspace.cast`, `ptr_offset_from`, `raw_eq` |
| Memory | `memcpy`, `memmove`, `memset`, `memcmp`, `prefetch.read`, `prefetch.write` |
| Float math | `sqrt`, `abs`, `fma`, `copysign`, `min`, `max`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `exp2`, `log`, `log2`, `log10`, `pow`, `floor`, `ceil`, `trunc`, `round` |
| GC barriers | `gc.write_barrier` |
| Control | `breakpoint`, `return_address`, `frame_address` |
| Branch hints | `expect` |
| Optimization | `black_box` |

### Canonical operation boundaries

Canonical MIR keeps vectors and tensors as first class instruction families because they carry target independent semantics that optimizers should preserve before lowering.
Vectors are the fixed width SIMD layer and stay deliberately small: splat, extract, insert, shuffle, select, compare, reduce, and convert.
Tensors are the structured N dimensional value layer and stay semantic rather than target flavored: shape, view, broadcast, transpose, pad, reduce, dot, convolution, gather, scatter, and related structural transforms.
Backend or vendor specific operations such as warp, subgroup, matrix core, or async copy primitives do not belong in canonical MIR.
Those belong in later target lowering layers once MIR has already preserved the portable vector, tensor, memory, and control semantics.


## Types

MIR types are concrete and fully resolved (no generics, no inference).
Struct type nodes carry field order and field types, and canonical physical layout lives in the layout table metadata.
Layout is computed against a target data layout.
This makes MIR machine-level while remaining target flexible.

Copyability encodes whether values are trivial or linear.
Aggregate types store copyability explicitly to avoid recomputation.

## Value and Memory Semantics

MIR has two ways to work with aggregates (structs, tuples, arrays): value semantics and memory semantics.

**Value operations** treat aggregates as immutable SSA values.
```mir
v0: (i32, i32) = tuple v1, v2         // construct a tuple
v3: i32 = field.get v0, 0             // extract first element (new SSA value)
v4: (i32, i32) = field.set v0, 1, v5  // "update" creates new tuple value
```

**Memory operations** compute addresses for load and store:
```mir
v0: ref<borrowed i32> = field.addr v1, 0   // get address of field 0
v2: i32 = load v0                          // load through pointer
store v0, v3                               // store through pointer
```

### Vectors and Tensors

Vector types represent fixed-width SIMD values to model SIMD lane registers, while tensors model N-dimensional value semantics for accelerator-friendly optimization.
 - `vector<T, N>` denotes an element type `T` and lane count `N`.
 - `tensor<T, [d0, d1, ...]>` for a tensor of element type `T` and ranked shape.

Tensor view types represent reference-like views into tensor-shaped memory.
Use `tensor_ref<kind addrspace(space) readonly T, [d0, d1, ...], layout=...>` in MIR text.
The `kind` is one of `managed`, `owned`, `borrowed`, or `raw`.
The `readonly` marker and `addrspace(...)` clause follow the same rules as `ref<...>` syntax.

Canonical MIR keeps vectors small and target independent.
The core vector family is lane-oriented and covers splat, extract, insert, shuffle, select, compare, reduce, and convert.
Backend-specific subgroup, warp, matrix-core, or vendor vector operations do not belong in canonical MIR.

Canonical MIR keeps tensors semantic and target independent.
`tensor.reshape` preserves element order and only changes the ranked shape view.
`tensor.broadcast` has pure expansion semantics and does not imply a specific materialization strategy.
`tensor.transpose` applies an explicit axis permutation.
`tensor.view` is a view-only operation over tensor-shaped memory and does not copy.
`tensor.slice`, `tensor.pad`, and `tensor.concat` have exact offset, extent, stride, and axis semantics.
`tensor.compare` and `tensor.select` are elementwise.
`tensor.reduce` preserves the declared reduction axes and operator semantics, and backends may only reorder reductions when the selected operation semantics allow it.
`tensor.dot` and `tensor.convolution` preserve explicit contraction, window, padding, dilation, and result-shape semantics.
`tensor.gather` and `tensor.scatter` must preserve the operation's explicit indexing mode and do not permit backend-specific out of bounds or duplicate-index behavior to leak into canonical MIR semantics.
Target-specific accelerator operations stay out of canonical MIR and belong in later lowering layers.

### Pointers

Pointer sized integer types are modeled explicitly.
`isize` is a signed integer with the target pointer width.
`usize` is an unsigned integer with the target pointer width.
Their concrete widths are resolved from module data layout metadata.

### Function Values

Bare code pointers use `fn(...) -> ...`.
Callable closure values use `fnvalue<fn(...) -> ..., env_type>`.
The signature operand is always a bare function pointer type, and the environment operand carries the captured context reference.
Canonical MIR treats function values as a dedicated two field aggregate rather than an anonymous struct convention.

### References

References carry a kind _and_ mutability:
- `managed` for auto-managed references
- `owned` for explicit ownership (`^T`)
- `borrowed` for `&T` and `&readonly T`
- `raw` for "unsafe" pointers

Reference syntax spells out the kind and mutability.
Address spaces are optional and appear after the kind.

| Kind | Mutability | Example | Meaning |
| --- | --- | --- | --- |
| managed | mutable or readonly | `ref<managed @T>`, `ref<managed readonly @T>` | GC-managed reference |
| owned | mutable or readonly | `ref<owned @T>`, `ref<owned readonly @T>` | owned reference for `^T` |
| borrowed | mutable | `ref<borrowed @T>` | mutable borrow (`&T`) |
| borrowed | readonly | `ref<borrowed readonly @T>` | readonly borrow (`&readonly T`) |
| raw | mutable | `ref<raw @T>` | raw pointer (mutable) |
| raw | readonly | `ref<raw readonly @T>` | raw pointer (readonly) |

Borrowed references are safe aliases verified by the borrow check pass.
Borrows are created by `field.addr`, `element.addr`, and by calls that return borrowed references with lifetimes.
A borrow ends when the reference value is no longer live.
Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
Derived borrows preserve provenance roots from their base value.
Merged borrows may carry the union of multiple provenance roots.
The MIR `Lifetime` model summarizes borrow provenance roots for function contracts rather than encoding the full internal borrow graph directly.
Dropping, freeing, or moving an origin invalidates every borrow rooted in it.
Dropping or freeing a value while it is borrowed is always an error.
Conflicting borrows and invalidating stores are MIR errors.
Raw references are unsafe pointers with no borrow tracking.
Raw references may be null or dangling and allow pointer arithmetic.
Crossing through raw pointers may force conservative provenance and liveness reasoning.
Deref and mutation use explicit `load` and `store` instructions.

Nullable references use `ref?<...>` with the same kind and mutability rules.
Mutability can be encoded for any reference kind.
`readonly` constrains mutation through that reference and is not deep or transitive immutability of the pointee graph.
Mutability is preserved as part of MIR type identity for all reference kinds.
Canonical formatting preserves mutable versus readonly spellings and does not force managed or owned references to readonly.

Address spaces describe where the reference points:
`generic`, `stack`, `global`, `shared`, `local`, `constant`, or a target-specific id.
Use `addrspace(name)` or `addrspace(7)` in the reference syntax:

```mir
ref<raw addrspace(shared) i32>
ref<raw addrspace(7) readonly i32>
```

`managed` references always use `addrspace(generic)`.
`borrowed`, `raw`, and `owned` references may use non generic address spaces when the target and allocator semantics define them.
`addrspace(constant)` references are always immutable.
`owned` references cannot use `addrspace(constant)`.
`addrspace(generic)` is the default and is omitted in canonical MIR formatting.
Address space changes are explicit and use the `addrspace.cast` intrinsic.

Field names are optional in MIR types and are for readability only:

```mir
type @Point = { x: f32, y: f32 }
```

### Type Metadata and Dispatch

Type facts capture layout, lineage, runtime identity, and dispatch structure for nominal types.
These facts are stored explicitly on `NodeTree.type_table` rather than in one metadata bag.
Layouts store size, alignment, stride, and field offsets in declaration order.
Lineage tracks parent types, interfaces, and sealed or final flags.
`TypeDescriptor` is the canonical runtime type identity object in MIR.
`TypeId` is a compact runtime identity token for lowered fast paths and runtime representations.
Canonical MIR uses `type_of` to produce `TypeDescriptor` values.
`TypeDescriptor` is also the canonical root for runtime scan, layout, and dispatch metadata queries.
`TypeTable.layout_by_type`, `lineage_by_type`, `union_layout_by_type`, `descriptor_by_type`, `vtable_by_type`, and `itabs_by_type` are the canonical per-type fact maps.
`display_name_by_type` and `field_map_by_type` are convenience lookup maps, not semantic sources of truth.

Dispatch tables describe vtables and itabs with slot ordering and targets.
Dispatch tables are stored in `NodeTree.dispatch_table.vtables` and `NodeTree.dispatch_table.itabs`.
Interface slot schemas are stored in `NodeTree.dispatch_table.interface_dispatch_shapes`.
VTables are only emitted for classes that require virtual dispatch.
Interface dispatch uses itabs for both struct and class implementations.
Each itab is specific to a (Type, Interface) pair.
Concrete type facts store a direct interface to itab map for fast lookup.
Itab slots include field offsets and method targets in interface declaration order.
Dynamic dispatch is represented explicitly as `call.virtual` and `call.interface` in both instruction and terminator form, plus tailcall variants.
Lowering those operations to `call` or `call.indirect` is a later optimization and codegen legalization decision.
Additional devirtualization facts are stored out of line in `NodeTree.dispatch_table.callsite_metadata`.
Callsites are keyed by `CallSite::Instruction` and `CallSite::Terminator`.
This metadata is sparse and optional.
VTables currently use global backing storage.
ITabs currently use immediate handle storage keyed by `ItabId`.

Interface inheritance flattens base interfaces in extends list order before local members.
Members inherited with the same name and signature reuse the first slot.
Vtables and itabs carry a `TypeDescriptor` entry as their canonical runtime type identity prefix.
Type descriptors link types to runtime metadata globals when needed.
Field maps provide name to field lookups for property access specialization.
Struct layouts describe value payloads with no identity semantics.

Volatile memory behavior is modeled on memory access metadata attached to `load`, `store`, and other memory-touching operations rather than through dedicated volatile intrinsics.
Class instance types are represented as `ref<managed @Payload>` or `ref<managed readonly @Payload>` where `@Payload` is the class field layout.
Dispatch metadata is stored out of line, and polymorphic classes include a vtable pointer in the payload layout when dynamic dispatch remains.
Boxing a value is represented as `managed.alloc` of the payload layout followed by `store` of the value.
`managed.alloc` defines semantic managed allocation only.
Canonical MIR does not commit to one collector, object header scheme, or managed reference representation.
Native collectors, VM heaps, and WasmGC backends are all valid implementations of MIR managed allocation.
`immortal_heap` is a distinct semantic region for process-lifetime managed storage and is not required to use the same allocation or collection strategy as `managed_heap`.

### Type Aliases

The MIR text format supports named type aliases for readability.
Aliases are purely syntactic sugar over concrete layouts.

```mir
type @Point = { x: i32, y: i32 }

function @use_point(v0: ref<managed @Point>) -> ref<managed @Point> {
block0(v0: ref<managed @Point>):
    return v0
}
```

Aliases are referenced with `@Name` in type positions.
The underlying MIR still stores and uses the concrete type.
This makes some documentation and tests much more readable.

## Functions

Functions contain basic blocks forming a CFG.
Imported functions have no body (`entry: None`, empty blocks).
The `allocation` field lets us mark functions as realtime-safe (no GC) or embedded-safe (stack only).
The `execution_model`, `execution_stage`, and `workgroup_size` fields mark GPU kernels and their dispatch shape when applicable.
The execution model identifies the pipeline category: `kernel`, `graphics`, or `ray_tracing`.
The execution stage identifies the stage within that pipeline, such as `vertex`, `fragment`, `task`, `mesh`, or `raygen`.
Workgroup size accepts one to three dimensions; omitted dimensions default to `1`.
The MIR text format accepts Rust-style attributes before items.

```mir
#[execution_model(kernel)]
#[workgroup_size(8, 1, 1)]
function @kernel() -> void {
block0:
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
block0:
    v0: i32 = call @compute_next() -> fn() -> i32
    yield v0, block1

block1(v1: i32):    // resumed with value from .next(arg) or resolved promise
    ...
```

The `CoroutineKind` (Generator, Async, AsyncGenerator) tells codegen what wrapper to generate.
Resume arguments appear as normal block arguments, with the resumed value appended after them.


## Attributes

Attributes can be attached to functions, globals, type aliases, and struct fields.
They use Rust-style syntax and support bare, value, and key-value forms.
Attributes always apply to the next item or field in the text stream.
The supported forms are `#[name]`, `#[name(value)]`, and `#[name(key=value, ...)]`.
Attribute values support identifiers, integers, floats, strings, and lists.

```mir
#[execution_model(graphics)]
#[execution_stage(vertex)]
function @vertex_main() -> void {
block0:
    return
}

#[packed]
type @Point = { #[offset(0)] x: i32, #[offset(4)] y: i32 }

#[section(".rodata")]
global @Message: ref<managed readonly @String> = "hello" ; readonly
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
