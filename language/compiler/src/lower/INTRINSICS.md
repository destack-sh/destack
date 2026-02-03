# Intrinsics

Intrinsics are primitive operations that Lower emits as MIR instructions or inline sequences rather than function calls.
Each backend (VM, Cranelift, WASM) implements these directly.

See `language/mir/src/tree/intrinsic.rs` for the canonical definitions.

## Overview

Intrinsics fall into several categories:

| Category | Purpose | Comptime? |
|----------|---------|-----------|
| Reflection | Type info queries | Yes (evaluated at compile time) |
| Bit manipulation | Low-level bit ops | No |
| Arithmetic | Checked/unchecked/saturating math | No |
| Memory | Bulk memory ops | No |
| Atomics | Thread-safe operations | No |
| GC | Garbage collection barriers | No |
| Float math | Math library functions | No |
| Control | Debugging and hints | No |
| SIMD | Vector operations | No |

## Reflection (Comptime Only)

These are evaluated during compilation and replaced with constants.
The VM handles them; native codegen never sees them.

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `type_of` | `(T) -> Type<T>` | Get the type descriptor of a value |
| `size_of` | `() -> usize` | Get the size of a type in bytes |
| `align_of` | `() -> usize` | Get the alignment of a type in bytes |

## Bit Manipulation

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `clz` | `(T) -> T` | Count leading zeros |
| `ctz` | `(T) -> T` | Count trailing zeros |
| `popcnt` | `(T) -> T` | Population count (count set bits) |
| `byte_swap` | `(T) -> T` | Byte swap (endianness conversion) |
| `bit_reverse` | `(T) -> T` | Reverse all bits |
| `rotate_left` | `(T, T) -> T` | Rotate bits left |
| `rotate_right` | `(T, T) -> T` | Rotate bits right |

## Arithmetic

### Checked (returns result + overflow flag)

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `add.overflow` | `(T, T) -> (T, bool)` | Add with overflow detection |
| `sub.overflow` | `(T, T) -> (T, bool)` | Subtract with overflow detection |
| `mul.overflow` | `(T, T) -> (T, bool)` | Multiply with overflow detection |

### Unchecked (UB on overflow)

These allow the optimizer to assume (or rather, pretend) no overflow ever occurs.

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `add.unchecked` | `(T, T) -> T` | Add (UB on overflow) |
| `sub.unchecked` | `(T, T) -> T` | Subtract (UB on overflow) |
| `mul.unchecked` | `(T, T) -> T` | Multiply (UB on overflow) |
| `div.unchecked` | `(T, T) -> T` | Divide (UB on zero or overflow) |
| `rem.unchecked` | `(T, T) -> T` | Remainder (UB on zero or overflow) |
| `shl.unchecked` | `(T, T) -> T` | Shift left (UB if shift >= bit width) |
| `shr.unchecked` | `(T, T) -> T` | Shift right (UB if shift >= bit width) |

### Saturating (clamps to min/max)

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `add.sat` | `(T, T) -> T` | Saturating add |
| `sub.sat` | `(T, T) -> T` | Saturating subtract |

### Saturating float to int conversions

These intrinsics map to MIR cast operators and never trap.
NaN converts to 0 and out of range values clamp to bounds.

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `fcvt_to_sint.sat` | `(F) -> T` | Saturating float to signed integer |
| `fcvt_to_uint.sat` | `(F) -> T` | Saturating float to unsigned integer |

## Memory

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `memcpy` | `(dst, src, len) -> ()` | Copy memory (non-overlapping) |
| `memmove` | `(dst, src, len) -> ()` | Move memory (handles overlap) |
| `memset` | `(dst, val, len) -> ()` | Set memory to byte value |
| `memcmp` | `(ptr, ptr, len) -> i32` | Compare memory regions |
| `volatile.load` | `(ptr<T>) -> T` | Volatile load (not optimized away) |
| `volatile.store` | `(ptr<T>, T) -> ()` | Volatile store (not optimized away) |
| `prefetch.read` | `(ptr) -> ()` | Prefetch for reading (CPU hint) |
| `prefetch.write` | `(ptr) -> ()` | Prefetch for writing (CPU hint) |

## Type Punning and Pointer Ops

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `transmute` | `(T) -> U` | Reinterpret bytes as different type |
| `ptr_offset_from` | `(ptr, ptr) -> isize` | Byte offset between pointers |
| `raw_eq` | `(T, T) -> bool` | Byte-wise equality comparison |

### Transmute Safety

`transmute<T, U>(value: T) -> U` reinterprets the bytes of `value` as type `U`.
This is inherently unsafe and bypasses the type system.

**Requirements (undefined behavior if violated):**
- `sizeof(T) == sizeof(U)` - sizes must match exactly
- `alignof(U) <= alignof(T)` - destination alignment must not exceed source
- Both types must be `Copy` (no drop glue, no managed references)

**Allowed transmutes:**
- `int64` ↔ `float64` (same size, both primitive)
- `[uint8; 4]` ↔ `uint32` (same size, both value types)
- `&T` → `&U` where layouts are compatible

**Forbidden transmutes (compile error or UB):**
- Anything involving managed references (GC pointers are opaque)
- Types with different sizes
- Creating invalid enum discriminants
- Creating invalid UTF-8 in strings

The compiler checks size and alignment at compile time.
Other invariants (valid discriminants, UTF-8) are the programmer's responsibility.

## Garbage Collection

Lower inserts write barriers for GC-enabled targets.
The specific GC algorithm is runtime-dependent; the intrinsic is a hook.

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `gc.write_barrier` | `(ptr, val) -> ()` | Write barrier for concurrent marking |

### Write Barrier Insertion

Lower inserts `gc.write_barrier` before every store to a managed reference field:

```mir
; obj.field = newValue (where field is a managed reference)
v0 = field.addr obj, fieldIndex
intrinsic.gc.write_barrier(v0, newValue)
store v0, newValue
```

The barrier shades the new value grey for concurrent marking.
This follows Go's approach: insertion write barriers with a concurrent mark phase.

### Weak References

Weak references (`WeakRef<T>`, `WeakMap<K, V>`, `WeakSet<T>`) are **well-known library types**, not intrinsics.
This follows the JVM/CLR/V8 pattern where these are recognized types with target-specific lowering.

Lower recognizes these well-known symbols and emits appropriate code for the target's memory model:

| Target | Lowering Strategy |
|--------|-------------------|
| GC | Coordinate with GC weak table; cleared during collection |
| Refcount | Weak reference counting (like Rust's `Weak<T>`) |
| Manual | Explicit weak handle API or unsupported |

```ds
// WeakRef<T> is defined in builtin/lib/native/
// Lower recognizes this well-known type
class WeakRef<T> {
    deref(): T | null { ... }
}
```

**Clearing semantics:** Weak references are cleared during GC collection, not immediately when the last strong reference is dropped.
This matches JavaScript's `WeakRef` behavior: `deref()` may return a value for some time after the object becomes unreachable.

For GC targets, when an object becomes unreachable (ignoring weak refs), the runtime:
1. During the mark phase, weak refs are not traversed (don't keep referent alive)
2. During the sweep/collection phase, weak refs to unreachable objects are cleared
3. WeakMap/WeakSet entries where the key was collected are removed
4. The object is collected

**Determinism:** Weak reference clearing is non-deterministic from the program's perspective.
Code must not rely on when `deref()` starts returning `null`.
Use strong references or explicit cleanup patterns when deterministic lifetime is needed.

No intrinsics needed; Lower handles these like other well-known types (`String`, `Array<T>`, `Promise<T>`).

## Atomics

Atomic operations require explicit ordering and scope metadata.
Ordering, scope, memory scope, and semantics must be compile time constants.

| Ordering | Description |
|----------|-------------|
| `relaxed` | No ordering constraints (weakest) |
| `acquire` | Reads can't be reordered before this |
| `release` | Writes can't be reordered after this |
| `acq_rel` | Both acquire and release |
| `seq_cst` | Sequentially consistent (strongest) |

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `atomic.load` | `(ptr<T>, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Atomic load |
| `atomic.store` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> ()` | Atomic store |
| `atomic.cas` | `(ptr<T>, expected, new, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> (T, bool)` | Compare-and-swap, returns old value and success flag |
| `atomic.cas.weak` | `(ptr<T>, expected, new, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> (T, bool)` | Weak compare-and-swap (may fail spuriously) |
| `atomic.xchg` | `(ptr<T>, value, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Exchange, returns old value |
| `atomic.fetch.add` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-add, returns old value |
| `atomic.fetch.sub` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-subtract, returns old value |
| `atomic.fetch.and` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-bitwise-and, returns old value |
| `atomic.fetch.or` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-bitwise-or, returns old value |
| `atomic.fetch.xor` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-bitwise-xor, returns old value |
| `atomic.fetch.min` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-min (signed), returns old value |
| `atomic.fetch.max` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-max (signed), returns old value |
| `atomic.fetch.umin` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-min (unsigned), returns old value |
| `atomic.fetch.umax` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-max (unsigned), returns old value |
| `atomic.fetch.fadd` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-add (float), returns old value |
| `atomic.fetch.fmin` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-min (float), returns old value |
| `atomic.fetch.fmax` | `(ptr<T>, T, order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> T` | Fetch-and-max (float), returns old value |
| `atomic.fence` | `(order, scope, memoryScope, locations, isVolatile, isMakeAvailable, isMakeVisible) -> ()` | Memory fence/barrier |

## Float Math

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `sqrt` | `(T) -> T` | Square root |
| `abs` | `(T) -> T` | Absolute value |
| `fma` | `(T, T, T) -> T` | Fused multiply-add: (a * b) + c |
| `copysign` | `(T, T) -> T` | Copy sign from one float to another |
| `min` | `(T, T) -> T` | IEEE 754 minimum |
| `max` | `(T, T) -> T` | IEEE 754 maximum |
| `sin` | `(T) -> T` | Sine |
| `cos` | `(T) -> T` | Cosine |
| `tan` | `(T) -> T` | Tangent |
| `asin` | `(T) -> T` | Arc sine |
| `acos` | `(T) -> T` | Arc cosine |
| `atan` | `(T) -> T` | Arc tangent |
| `atan2` | `(T, T) -> T` | Arc tangent of y/x |
| `exp` | `(T) -> T` | e^x (natural exponential) |
| `exp2` | `(T) -> T` | 2^x |
| `log` | `(T) -> T` | Natural logarithm |
| `log2` | `(T) -> T` | Base-2 logarithm |
| `log10` | `(T) -> T` | Base-10 logarithm |
| `pow` | `(T, T) -> T` | Power: base^exponent |
| `floor` | `(T) -> T` | Round toward negative infinity |
| `ceil` | `(T) -> T` | Round toward positive infinity |
| `trunc` | `(T) -> T` | Round toward zero |
| `round` | `(T) -> T` | Round to nearest, ties to even |

## Control Flow and Debugging

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `unreachable` | `() -> !` | Mark code as unreachable (UB if executed) |
| `breakpoint` | `() -> ()` | Trigger debugger breakpoint |
| `abort` | `() -> !` | Abort execution immediately |
| `return_address` | `() -> ptr` | Get return address of current function |
| `frame_address` | `() -> ptr` | Get frame pointer of current function |
| `expect` | `(bool, bool) -> bool` | Hint expected value of condition |
| `likely` | `(bool) -> bool` | Hint condition is likely true |
| `unlikely` | `(bool) -> bool` | Hint condition is likely false |
| `black_box` | `(T) -> T` | Optimization barrier |

## SIMD

SIMD follows a Zig-like model: vectors are first-class types with static lane counts.
Standard operators (+, -, *, etc.) work element-wise on vector types.
Lower maps these to target SIMD instructions when available.
If the target lacks support, Lower scalarizes to loops.

### Vector Type

Vectors are represented as `Vector<T, N>` where T is the element type and N is the lane count:

```mir
type @Vec4f32 = Vector<f32, 4>   ; 4-lane f32 vector
type @Vec8i32 = Vector<i32, 8>   ; 8-lane i32 vector
```

### SIMD Intrinsics

| Intrinsic | Signature | Description |
|-----------|-----------|-------------|
| `shuffle` | `(Vector<T, N>, Vector<T, N>, Vector<int, M>) -> Vector<T, M>` | Shuffle lanes according to mask |
| `select` | `(Vector<boolean, N>, Vector<T, N>, Vector<T, N>) -> Vector<T, N>` | Per-lane conditional select |
| `splat` | `(T) -> Vector<T, N>` | Broadcast scalar to all lanes |
| `reduce.add` | `(Vector<T, N>) -> T` | Horizontal sum of all lanes |
| `reduce.mul` | `(Vector<T, N>) -> T` | Horizontal product of all lanes |
| `reduce.min` | `(Vector<T, N>) -> T` | Minimum of all lanes |
| `reduce.max` | `(Vector<T, N>) -> T` | Maximum of all lanes |
| `reduce.and` | `(Vector<T, N>) -> T` | Bitwise AND of all lanes |
| `reduce.or` | `(Vector<T, N>) -> T` | Bitwise OR of all lanes |
| `reduce.xor` | `(Vector<T, N>) -> T` | Bitwise XOR of all lanes |

Lower currently wires `splat`, `select`, and `reduce.*` to MIR vector instructions.
The `shuffle` intrinsic is reserved but not lowered yet.

### Scalarization Fallback

Lower does not yet scalarize vector operations, and the example below is a planned fallback path.

When the target lacks SIMD support (or the vector width exceeds hardware capabilities), Lower scalarizes vector operations to scalar loops:

```mir
; Vector<f32, 4> add without SIMD hardware
v0 = element.get a, 0
v1 = element.get b, 0
v2 = fadd v0, v1
; ... repeat for all lanes
v8 = aggregate Vector<f32, 4> (v2, v3, v4, v5)
```

The optimizer may later vectorize these if profitable.

### Future: GPU/Shader Support

SIMD is the foundation for accelerated computing.
Future work includes:
- Compute shaders via SPIR-V/Metal/WebGPU backends
- GPU kernel extraction from annotated functions
- Automatic parallelization of map/reduce patterns

These will build on the same vector type infrastructure.
