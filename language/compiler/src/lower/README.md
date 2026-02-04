# Lower

Lower runs after Elaborate and turns high-level "canonical" DIR into low-level, target-specific MIR.

## Objectives

The _dream_ is **Rust performance with TypeScript ergonomics**.
Of course, performance and ergonomics are in some tension, so this isn't fully achievable without breaking the things that make TypeScript great.
We want to enable *up to* Rust performance with some additional constructs while improving modern TS performance to around Go/C#-level predictable performance without _requiring_ additional changes. 
So, basically we want:
- **Best case (target):** Rust-tier performance (zero-cost abstractions, no GC pauses)
- **Average case (target):** Go-tier performance (efficient GC, good concurrency)
- **Worst case (target):** Competitive with optimized JS runtimes (V8, JSC, SpiderMonkey)

AOT compilation provides predictable performance without warmup; however, astounding engineering efforts have already gone into making modern JS engines' speculative optimization approximate (or even beat!) static compilation on common "dynamic" patterns.
That said, today nobody would seriously consider writing "systems software" in JS/TS, which is a shame, because modern TS is actually a fantastic language for _full_-stack software.

Our advantage is consistency and control, and, of course, you don't need to ship a JS runtime.
We try to keep TS semantics as much as possible, but there are some tradeoffs and additional strictness requirements to make TS sound for AOT compilation. (These are noted in the relevant sections below.)

## Pipeline

Lower receives patched canonical DIR post-Execute and produces target-specific MIR.

```
... Analyze ───► Elaborate ───► Execute ───► Lower ───► Optimize ───► Generate ───► ...
```

### Input: Canonical DIR

Lower receives "canonical" typed DIR after Analyze and Elaborate (and Execute).
Lower expects every expression to have a recorded type in the TypeTable, and missing types are compiler bugs that must error.
- Desugaring complete (e.g., `+=` → `+` and assign)
- Patterns expanded to decision trees
- Types fully inferred for all expressions ("Types")
- Resolutions resolved ("Resolutions")
- Static parameters resolved to values ("StaticExpression")
- Polymorphic instances created ("Instances")
- Comptime blocks executed and results patched in ("Execute")

### Output: Target-Specific MIR

MIR is generated per-target with target-specific decisions:
- Memory layout (LP64, ILP32, etc.)
- Calling conventions (C, System, etc.)
- Alignment requirements
- Policy-controlled checks (bounds, overflow, etc.)

## Implementation Structure

Lower organizes the implementation by responsibility.

- `module/`: orchestration and caching
- `type/`: type lowering and layout (including nominal field layouts)
- `item/`: declaration lowering (globals, functions, methods)
- `table/`: dispatch tables (vtables, itabs) and RTTI
- `emit/`: function body lowering (statements, values, control)

Nominal layouts are computed from declared fields and cached on demand.

## Layout Map

Lower treats layout as a **queryable, cached graph** instead of a monolithic pass.
The goal is to keep representation decisions explicit and local.
Lower can answer "what is the layout of this type?" at any point during lowering.

### Layout Categories

We model layout in two layers:
- **Shape**: logical fields, tags, tables, and constraints.
- **Placement**: concrete offsets, alignment, size class, inline vs boxed decisions.

This keeps unions, intersections, and interfaces manageable.
Their shape often exists without a single concrete placement.

| Category | Shape | Placement | Notes |
| --- | --- | --- | --- |
| Scalar/Immediate | scalar value | fixed width | includes tagged pointer immediates |
| Pointer | address | pointer-sized | heap-managed or external |
| Tuple | ordered fields | packed/offset fields | homogeneous is still tuple |
| Struct | named fields | packed/offset fields | nominal, value semantics |
| Class | instance fields | pointer + optional vtable | reference semantics |
| Array/Slice | element + length | header + data | policy: inline vs heap |
| Function | signature | pointer or fat pointer | closure values pair fn pointer with env pointer |
| Tagged Union | tag + payload | inline or boxed | tag value + payload layout |
| Untagged Union | set of layouts | external discrimination | RTTI or caller-provided tag |
| Interface | dispatch surface | itab/vtable + data | separate dispatch layout |
| Intersection | composed view | no new storage | layout = primary + itabs |

### Union Strategy

Union layout is chosen per union:
- **Inline tagged**: tag + payload in one block (size ≤ 2×ptr size)
- **Boxed tagged**: tag + pointer to payload
- **Untagged**: no tag, relies on RTTI or external discriminant

`null` and `undefined` are distinct union elements with distinct tags.
The only special case is a union of a single reference type plus `null`, which lowers to a nullable reference.

### Dispatch Layout (VTables/ITabs)

Dispatch layout is modeled separately from data layout.
- **VTable**: class method table for virtual dispatch.
- **ITab**: interface table for structural or nominal interface dispatch.

A type layout can reference zero or more dispatch layouts, but dispatch layouts never alter the data placement.
VTables are only emitted for classes that still require virtual dispatch after devirtualization.
Interface dispatch always uses itabs, even when the concrete type is a class.

### String Equality

`===` on strings is **value equality** (content comparison), not reference equality.
String interning is an optimization detail with no semantic guarantees.

```ds
const a = "hello";
const b = "hel" + "lo";
a === b  // true (same content)
```

### Symbol Identity

- `Symbol("desc")` creates a **unique** symbol each call
- `Symbol.for("key")` returns the **same** symbol for a given key (global registry)
- `===` compares symbol identity (interned integer comparison)

```ds
Symbol("a") === Symbol("a")        // false (unique each call)
Symbol.for("a") === Symbol.for("a") // true (same from registry)
```

**Registry scope:** The `Symbol.for()` registry is global across all modules in a compilation unit.
If module A calls `Symbol.for("key")` and module B calls `Symbol.for("key")`, they get the same symbol.
This matches JavaScript's global symbol registry behavior.

### Property Enumeration

Object properties enumerate in **insertion order**, matching ES2015+.
This applies to `Object.keys()`, `for...in`, and RTTI field iteration.

### Default Arguments

Default argument expressions evaluate **per-call** when the argument is `undefined`.
Evaluation occurs in the function's scope, not at definition time.

```ds
function log(timestamp = Date.now()) { ... }
log()  // evaluates Date.now() on this call
log()  // evaluates Date.now() again (different value)
```

## Native Runtime Library

Lower doesn't special-case types like `String` or `Array<T>`.
These are defined in `language/builtin/lib/native/` as regular Destack structs (with some intrinsics).
Lower treats them basically like any other user-defined type.
The native and JS builtins live here:

<pre>
language/builtin/
├── core/       # Operator interfaces (Add, Index, etc.) - compiler desugars to these
├── std/        # Universal extensions
└── lib/
    ├── native/ # Native target types: String, Array, Map, etc.
    ├── es/     # JS target types (uses JS-defined built-ins)
    └── ...
</pre>

---

# Data Model

This section defines the semantic model for lowering: how high-level concepts map to low-level representations.

## Lowering Model

### Monomorphization

DIR has Instance-level information from Analysis, but is still polymorphic.
For native targets, we fully monomorphize these Instances into concrete types with known sizes and offsets.
(Thus, each generic instantiation gets its own specialized MIR logic.)

```ds
function identity<T>(x: T): T { x }

identity<int32>(5)      // generates: identity_int32
identity<string>("hi")  // generates: identity_string
```

Monomorphization trades code size for runtime performance:

| Aspect | Benefit | Cost |
|--------|---------|------|
| No boxing | Zero overhead generics | More code copies |
| Known offsets | Direct field access | Larger binary |
| Inlining | Cross-generic optimization | Longer compile times |
| Register allocation | Optimal per-instantiation | More codegen work |

The DIR `Instance` type tracks generic instantiations with their static arguments.
Lower receives these `Instance`s and generates specialized MIR for each instantiation.
(For JS/TS codegen, we preserve polymorphic code with no monomorphization needed, except for
non-erasable value parameters like `const N: int`.)

### Name Mangling

Monomorphized functions need unique, deterministic names for linking.
We use a human-readable scheme (inspired by Rust and Zig):

**Format:** `@<module_path>.<type>.<method>__<type_args>__h<hash>`

**Examples:**
```mir
@std.collections.Map.get__string__i32__h7f9a3e1     // Map<string, i32>.get()
@myapp.models.User.getName__h1b2c3d4                // User.getName()
@myapp.utils.identity__Point__h2c3d4e5              // identity<Point>()
@core.ops.Add.add__Vec2__Vec2__h3d4e5f6             // Vec2's Add<Vec2>.add()
```

**Rules:**
- `@` prefix (like Zig/MIR convention)
- `.` separates module/type/method segments (valid MIR identifier char)
- `__` separates type arguments
- Module path included for uniqueness
- Type arguments appended for monomorphized generics
- Hash suffix `__h...` for exported/linkable symbols (matches Rust/Zig practice)

Examples omit the hash suffix for internal-only symbols to keep snippets readable.
This produces predictable, readable symbol names in debuggers and error messages.

### Resolution

Resolution tells Lower *which symbol* is being called (or might be called in dynamic resolution) at a call site.
This is determined by Analyze and attached to DIR nodes.
- **Resolution** answers: "Which declaration are we targeting?"
- **Dispatch** answers: "How do we invoke that target at runtime?"

Even with `Resolution::Static`, the target may require vtable dispatch if it's a virtual method on a polymorphic type.
Dynamic resolutions are reified into if-else chains with `is` type checks in the Elaborate phase; i.e., the Lower phase only sees `Resolution::Static` and `Resolution::Builtin`.
(`Resolution::Dynamic` is specifically for *union symbols* where different union variants call different target symbols; but, remember, symbols are polymorphic in DIR so this may be different MIR symbols even for the same DIR symbols).

#### Builtin Resolution

Primitive operations on builtin types.
No function call needed; Lower emits MIR instructions directly.

```ds
a + b  // builtin resolution -> int32 + int32
```

Lowers to:
```mir
v2 = iadd v0, v1
```

#### Static Resolution

Single known target symbol.
The *symbol* is known, but dispatch may still be virtual or direct depending on the method.

**Direct dispatch** (function, non-virtual method, final class, struct method):
```ds
user.getName()  // static resolution, non-virtual
```

Lowers to:
```mir
v1 = call @User.getName(v0)
```

**Virtual dispatch** (virtual method on class or interface):
```ds
node.update(delta)  // static resolution { target: Node::update }, but virtual
```

Lowers to vtable lookup:
```mir
v1 = field.get v0, 0           ; load vtable pointer from object layout
v2 = field.get v1, 2           ; load update method at vtable slot 2
v3 = call.indirect v2(v0, delta) -> fn(ref<raw void>, i32) -> void ; indirect call through vtable
```

The key: static resolution means we know *which method signature* (Node::update), but if it's virtual, the actual implementation depends on the concrete type.

#### Dynamic Resolution

Union-based dispatch: different union symbols call different target symbols.
Elaborate has already generated the symbol-dispatch logic; Lower just emits it.

**Union method dispatch:**
```ds
function render(obj: Mesh | Light) {
    // different implementations for Mesh.draw and Light.draw
    obj.draw(ctx);
}
```

Elaborate transforms to a type guard chain (conceptually `instanceof`/`T.is`), so Lower never has to deal with non-itab/vtable based dynamic dispatch directly.
In the `Mesh | Light` case we could also have used a `Drawable` interface or `Object3D` base type, and then this would be solved with vtable dispatch instead of dynamic resolution.

## Type Representation

All layouts here are post-monomorphization and target-specific.

### Primitives

| DIR Type | MIR Type | Native Representation |
|----------|----------|----------------------|
| `boolean` | `Boolean` | `i8` (0 or 1) |
| `int8..int64` | `Int { width, signed: true }` | Native integer |
| `uint8..uint64` | `Int { width, signed: false }` | Native integer |
| `float32`, `float64` | `Float { width }` | IEEE 754 float |
| `void` | `Void` | Zero-sized |

#### Integer Semantics

| Type | Width | Notes |
|------|-------|-------|
| `int` | 64-bit signed | Alias for `int64`, fixed across all platforms |
| `uint` | 64-bit unsigned | Alias for `uint64`, fixed across all platforms |
| `int32`, `uint32`, etc. | As named | Explicit width types |

**Rationale:** Fixed 64-bit default ensures predictable overflow behavior across platforms.
32-bit platforms are rare (<1% of modern targets).

### Overflow Behavior

Integer arithmetic has defined overflow semantics (unlike C, inspired by Zig and Rust).
Default overflow behavior is configurable per target in build configuration (`overflowChecks`) or via `safetyPreset`.
Explicit `overflowChecks` overrides any `safetyPreset` value.

When overflow checks are enabled, the lowerer emits explicit overflow checks for `+`, `-`, and `*` and traps on overflow.
When overflow checks are disabled, arithmetic wraps in two's complement.
Signed division and remainder trap on division by zero and on `min_value / -1`.
When safety checks are disabled, the lowerer emits `div.unchecked` and `rem.unchecked` intrinsics.

**Explicit operators:** Always available regardless of mode.
- `+%`, `-%`, `*%`: wrapping (two's complement wrap)
- `+|`, `-|`, `*|`: saturating (clamp to min/max)

```ds
const a: uint8 = 250;
const b: uint8 = 10;

a + b       // checked when overflowChecks enables safety
a +% b      // always wrapping: 250 + 10 = 4
a +| b      // always saturating: 250 + 10 = 255
```

MIR representation:
- `Binary` uses wrapping semantics for integer add, sub, and mul
- overflow checks use `add.overflow` and related intrinsics plus `check`
- unchecked operations use `*.unchecked` intrinsics

### Null and Undefined

TypeScript has both `null` and `undefined`, and native lowering preserves the distinction.
`null` is represented as a null reference for reference types and remains distinct from `undefined`.
`undefined` is represented only through tagged unions and never aliases `null` at runtime.

A union of a single reference type and `null` lowers to a nullable reference:
```ds
type MaybeRef<T> = T | null  // T is a reference type
// layout: ref?<managed T>
```

All other unions that include `null` or `undefined` lower to tagged union layouts:
```ds
type MaybeInt = int | null
type MaybeRefOrUndefined<T> = T | null | undefined
```

Optional parameters (`x?: T`) lower as `x: T | undefined` and remain explicit in the type system.

### BigInt

BigInt (`bigint` type, `42n` literals) is a **library type with operator overloading**.
It is not a compiler intrinsic; it's defined in `language/builtin/lib/native/` and uses the standard operator interfaces (`Add`, `Subtract`, `Multiply`, etc.).

For native, we use a tagged pointer representation with small-integer optimization.
(This is similar to what many JS runtimes do internally as well.)

| Case | Layout | Notes |
| --- | --- | --- |
| Small bigint (fits in 63 bits) | `[63-bit value][1-bit tag=1]` | inline, no heap allocation |
| Large bigint (> 63 bits) | `[pointer to limb array][1-bit tag=0]` | heap-allocated, arbitrary precision |

```ds
const small: bigint = 42n          // inline: 0x0000000000000055 (42 << 1 | 1)
const large: bigint = 2n ** 100n   // heap: pointer to limb array
```

The native `bigint` has TypeScript semantics:
- `===` compares values, not identity (bigints are value types semantically, same as JS/TS)
- Small bigints compare inline; large bigints compare limb-by-limb
- Overflow from small to large is automatic and transparent (same layout size)

**Operators:**
BigInt implements `Add<bigint>`, `Subtract<bigint>`, `Multiply<bigint>`, etc.
Operators lower to method calls: `a + b` → `a.add(b)`.

**Mixed operations:**
`bigint + int` requires explicit conversion. No implicit coercion between bigint and other numeric types.

**Division:**
`/` returns `bigint` (truncated). Use library methods for remainder, divmod.

### Symbol

TypeScript's `symbol` is a unique identifier.
For native, symbols are interned integers:

```ds
const symbol = Symbol("description")
```

MIR representation:
- `symbolId: uint64` (unique per-symbol, assigned at creation)
- Description string stored separately in symbol table

Symbol comparison is just integer comparison.
`Symbol.for()` looks up in a global string-to-symbol map.

### Strings

Strings are immutable byte sequences with target-specific payload encoding.
Native targets use UTF-8 payloads by default.
WASM and JS-interop targets use UTF-16 payloads to avoid boundary transcoding.

**Ownership semantics** (like Rust):

| Destack | Rust | Description |
|---------|-----------------|-------------|
| `string` | ~`Rc<str>` | GC-managed immutable string (default) |
| `^string` | `String` | Owned mutable string (manual/RAII) |
| `&string` | `&str` | Borrowed immutable view |

Most code uses `string` (GC-managed). Use `^string` for performance-critical code with explicit ownership.

#### String Layout

The header layout is identical across targets, and only the payload encoding changes.
UTF-8 payloads use `uint8` data and UTF-16 payloads use `uint16` data.

```ds
struct String {
    lengthUtf16: uint32;      // utf-16 code unit count for ts compatibility
    lengthBytes: uint32;      // byte length of utf-8 data
    hash: uint64;             // cached hash valid when HasHash is set
    capacity: uint32;         // allocated capacity in bytes
    flags: uint32;            // runtime metadata flags
    data: *uint8;             // utf-8 bytes
}
```

GC metadata and type descriptors are stored out of line (see [Managed Object Metadata](#managed-object-metadata)).
For TypeScript semantic compatibility:
- `.length` returns UTF-16 code unit count (not bytes, not codepoints)
- `.charAt(i)` indexes by UTF-16 code units
- ASCII-only strings (common case) use O(1) indexing
On UTF-16 targets, `lengthBytes` caches the UTF-8 byte length and is computed lazily.
The payload encoding is fixed per target configuration.
The `string` payload pointer type is `*uint8` on UTF-8 targets and `*uint16` on UTF-16 targets.
The `string` header uses `capacity` for owned `^string` growth and usually keeps `capacity == lengthBytes` for GC-managed `string`.
Flags are runtime metadata bits with stable meanings.
- `HasHash`: `hash` is populated and valid
- `IsAscii`: payload is ASCII-only
- `IsStatic`: payload is static read only data
- `IsInterned`: string content is interned
- `IsExternal`: payload is owned outside the managed heap

#### String Literals

String literals are interned at compile time in a read only data section:

```ds
const s = "hello"   // pointer to static data (never collected)
```

**Interning scope:** String literals are globally deduplicated within a compilation unit.
The same literal `"hello"` appearing in multiple modules resolves to the same address.
However, runtime-created strings are NOT guaranteed to be interned.
`"hel" + "lo"` at runtime creates a new string, even if `"hello"` exists as a literal.
String equality (`===`) is always value comparison, never reference comparison.

#### Template Literals

```ds
`Hello, ${name}!`
```

Lowers to string concatenation:
```mir
type @string = ref<struct { u32, u32, u64, u32, u32, *u8 }>

function @template_example(v0: @string) -> @string {
block0(v0: @string):
    v1 = global.const @str_Hello
    v2 = call @string.concat(v1, v0)
    v3 = global.const @str_Bang
    v4 = call @string.concat(v2, v3)
    return v4
}
```

### Arrays

Arrays are heap-allocated, dynamically-sized collections (like Rust's `Vec<T>`).
We also support fixed-size arrays which are more like Rust arrays / slices:
- **Fixed-size arrays** `T[N]` are inline values with no header
- **Dynamic arrays** `T[]` are heap-allocated, growable buffers

| Array kind | Layout | Notes |
| --- | --- | --- |
| `T[N]` | inline `elements: [T; N]` | size = `N * sizeof(T)` |
| `T[]` | header `{ length: uint32, capacity: uint32, data: T[capacity] }` | heap allocated, growable |

| Pattern | MIR Type | Notes |
|---------|----------|-------|
| `T[N]` | `Type::Array { element, length: N }` | Inline, value semantics |
| `T[]` | `ref<managed @Array<T>>` | Heap, reference semantics |
| `TypedArray` | `Type::Reference(kind: Raw)` to buffer | Direct memory access |

**Important:** `T[]` is NOT `Array<unknown>`. After monomorphization, we know T.
`Array<unknown>` boxes elements and uses RTTI for type checks.

**Array operations:**
```mir
; a[i] where a: int[], a is v0, i is v1
v2 = field.get v0, 2        ; get data pointer
v3 = element.get v2, v1     ; load element at index

; a.push(x) where x is v4
call @Array.push(v0, v4)

; a.length
v5 = field.get v0, 1        ; load length field
```

**Bounds checks:**
Array and slice indexing emits bounds checks by default.
The policy is configured per target (`boundsChecks` in `dsconfig.json` or via `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `boundsChecks` overrides any `safetyPreset` value.

**Null checks:**
Reference operations emit null checks when required.
The policy is configured per target (`nullChecks` or `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `nullChecks` overrides any `safetyPreset` value.

**Division checks:**
Division and remainder emit checks for division by zero (and signed `min_value / -1`) when enabled.
The policy is configured per target (`divisionChecks` or `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `divisionChecks` overrides any `safetyPreset` value.

**Shift range checks:**
Shift operations can emit range checks when enabled.
The policy is configured per target (`shiftChecks` or `safetyPreset`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)
Explicit `shiftChecks` overrides any `safetyPreset` value.

**Check failure behavior:**
When a check fails, the compiler can trap, panic, or abort.
The policy is configured per target (`checkFailure`):
- `trap`: emit a trap/unreachable
- `panic`: call the panic runtime (uses `panic`/`unwind` policy)
- `abort`: abort immediately
The safety preset does not modify `checkFailure`.

**Slices:**
Slices are explicit view types with a pointer and length (`Slice<T>`).
They are distinct from borrowing the array object itself.
Functions that accept a slice can take `Slice<T>` or `&Slice<T>`.
Borrowing an array object does not imply a slice view.

### Structs and Classes

Structs are value types and do not have reference identity.
Classes are reference types and carry identity.
Ownership modifiers (`^T`, `&T`) change storage and lifetime and never change identity semantics.
Lower may represent structs by reference when the observable semantics remain value based.

| Aspect | struct | class |
|--------|--------|-------|
| Reference identity | No (`===` is compile error) | Yes (`===` compares pointers) |
| Type identity | Via metadata or fat pointer when needed | Via vtable or metadata when needed |
| Equality | By value (`==` compares properties) | By reference (unless `Equal` implemented) |
| Extends | No | Yes |
| Implements | Yes | Yes |
| Virtual | None (all calls static) | Methods virtual by default |

**Virtual method dispatch for classes:**
- All class methods are virtual by default (like TypeScript/JavaScript prototype methods)
- Private methods (`#method`) use direct dispatch (not inheritable)
- **Devirtualization**: The optimizer analyzes the class hierarchy and converts virtual calls to direct calls when safe:
  - No subclasses exist in the compilation unit → direct call
  - Method not overridden by any subclass → direct call
  - Receiver type is exactly known (not a supertype) → direct call
- Vtables are only emitted when dynamic dispatch remains, fully devirtualized classes can omit vtables

The `final` keyword on methods or classes is an API contract ("you may not override/extend"), not an optimization hint.
For whole-program compilation, the optimizer already knows what's overridden.
`final` matters for libraries where downstream users could extend classes.

Both lower to nominal instance layouts with computed property offsets. The key difference is **reference identity**: classes have it (two instances with same data are still different objects), structs don't (two structs with same data are equal). Both can have **type identity** (RTTI) when needed for `instanceof`, `T.is`, or `typeOf`.

#### RTTI and Type Tags

RTTI (runtime type identity) is unified via `TypeTag` handles that point to `TypeDescriptor` values.
Polymorphic classes store a vtable pointer in the object layout for virtual dispatch.
If any class in a lineage requires virtual dispatch, every class in that lineage includes a vtable pointer at offset 0 so upcasts need no pointer adjustment.
Vtable slot 0 stores the `TypeTag` for fast `instanceof`, `T.is`, and `typeOf`.
Structs remain headerless and never store a vtable pointer.
Thin-pointer checks on structs recover `TypeTag` from GC metadata when needed.
Interface and `unknown` values carry `TypeTag` in fat pointers.
Class references are thin pointers, so the vtable pointer must live in the object layout when present.

GC metadata lookup only applies to managed references.
Non-managed values require explicit tags (union tags or fat pointers) or compile-time type knowledge.

RTTI is only emitted when runtime type checks are possible:
- Used with `instanceof`, `T.is`, or `typeOf` on unknown values
- Stored in `unknown`
- Used in runtime reflection
- Used in untagged unions that require runtime discrimination

#### Struct Layout

Structs have no **reference identity** (no `===`).
Structs are always headerless and use metadata or fat pointers for RTTI.

```ds
struct Point { x: float32, y: float32 }

struct PointLayout {
    x: float32,              // offset 0
    y: float32,              // offset 4
}
// size: 8 bytes
```

Structs are data-oriented: two structs with the same properties are equal by value (`==`) by default.
(Reference comparison (`===`) on structs is a compile error.)

**Prefer discriminated unions** for performance-critical code to avoid runtime RTTI lookups:

```ds
struct Circle { kind: "circle" = "circle", radius: float };
struct Rect { kind: "rect" = "rect", width: float, height: float };
type Shape = Circle | Rect;

function area(shape: Shape): float {
    match (shape.kind) {
        "circle" => 3.14159 * shape.radius * shape.radius
        "rect" => shape.width * shape.height
    }
}
```

String tags are interned to integers (see [String Tag Interning](#string-tag-interning)).

#### Class Layout

Polymorphic classes have vtable pointers for virtual dispatch and RTTI:

```ds
class Node {
    name: string;
    update(delta: float): void { }
}
```

Native layout:
```ds
struct NodeLayout {
    vtablePtr: &VTable,      // offset 0, vtable[0] = Node_TypeTag
    name: ref<string>,       // offset 8
}
```

Classes have both reference identity (`===` compares pointers) and type identity (via vtable or metadata).
Polymorphic classes store their vtable pointer because class references are thin pointers and dynamic dispatch is required.
(This is consistent with Java and C++ class objects while keeping struct layouts headerless like Go.)
Non-polymorphic classes omit the vtable pointer and use metadata or fat pointers for RTTI when needed.

Class layouts are static on native targets.
There are no hidden classes or runtime shape transitions.
Dynamic property addition must use explicit map/dictionary types.

#### Managed Object Metadata

Managed objects have no per object GC header.
GC metadata is stored out of line in allocator side tables, similar to Go.
TypeTag values are pointers to TypeDescriptor values, not integer ids.
Null references use 0x0 for the pointer value.
Undefined is represented through tagged unions, not pointer tagging.

Per span metadata includes:
- Mark bits for GC tracing
- Size class and allocation layout info
- A TypeTag per object for scanning and type queries

Polymorphic classes store a vtable pointer in the object for virtual dispatch and fast `instanceof`/`T.is`.
Structs remain headerless and rely on metadata or fat pointers for RTTI.

Tradeoffs:
- Predictable object layouts and smaller per object overhead
- Thin pointer RTTI queries require a metadata lookup

**Explicit ownership semantics:**
Use `^T` to request an owned reference (move-only):
```ds
function process(point: ^Point) {    // ^Point is owned, caller gives up ownership
    // point is now owned by this function
}
```

#### Field Layout Policy

Field layout is deterministic per target and part of the ABI.
The default layout minimizes padding by sorting fields by alignment and size.
Source order is the stable tie-break when alignment and size are equal.
Class layouts place parent fields first, then apply the same policy to new fields.
Use `@layout("C")` to match the C ABI for FFI.
Use `@layout("source")` to preserve declared order.

### Class Inheritance

Classes with `extends` get special handling for field layout and method dispatch.
Parent fields come first, then child fields.

```ds
class Node {
    name: string
}

class Sprite extends Node {
    texture: Texture
}
```

Sprite's MIR layout (polymorphic):
- offset 0: vtablePtr
- offset 8: name (from Node)
- offset 16: texture (from Sprite)

This ensures a `Sprite` pointer can be used where a `Node` pointer is expected.
Polymorphic classes use vtables for virtual methods (and non-virtual methods are direct calls).

```ds
class Node {
    update(delta: float): void { }
}

class Sprite extends Node {
    update(delta: float): void { this.animate(delta) }
}
```

### Tuples

Tuples lower to anonymous `Type::Struct` with indexed fields.

```ds
(int, string, bool)  →  Struct { fields: [i64, String, i8] }
```

### Associated Types

Associated types (like `Container<T>.Item`) are resolved at compile time during the Elaborate phase.
By the time Lower runs, all associated types have been replaced with their concrete types.
Lower never sees associated type references; it only sees the resolved concrete types.
(Sort of like we deal with Resolution::Dynamic.)

```ds
interface Container<T> {
    type Item = T
}

// In DIR after Elaborate:
// Container<int>.Item is already resolved to int
```

### Newtypes

Newtypes lower to nominal wrapper types with a single field that stores the underlying representation.
They keep a distinct MIR type identity for metadata, drop hooks, and extension methods.
Their layout matches the wrapped type, so optimizations can erase the wrapper when identity is not observed.

```ds
newtype UserId = int;
const id: UserId = UserId(42);  // lowers to: struct { value: int } with nominal type UserId
```

Pattern matching on newtypes unwraps the payload and can be optimized away:
```ds
match (id) {
    UserId(n) => print(n)
}
```

## Union Types

Destack supports TypeScript's structural unions.

### Discriminated Unions (Tagged)

When all variants share a discriminant field (e.g., `kind`), use inline tagging:

```ds
type Result<T, E> = { kind: "ok", value: T } | { kind: "err", error: E }
```

Lowers to a MIR struct with:
- `tag: uint8` (0 = ok, 1 = err)
- `payload: uint8[N]` (max of sizeof(T), sizeof(E))

The `tag` field is the discriminant.
Pattern matching becomes a switch on tag.

### Union Representation

Lower chooses union representation based on these rules (in order):
Union upcasts rely on explicit `Expression::Cast` nodes inserted by Elaborate.
When contextual typing assigns the union type to a concrete expression, Lower uses static resolution to select the correct union variant tag.

1. **Nullable reference** - A union of a single reference type and `null` lowers to a nullable reference.
   Examples:

   ```ds
   type MaybeString = string | null
   // layout: ref?<managed string>
   ```

2. **Inline tagged** - Total size ≤ 2×pointer_size (16 bytes on 64-bit).
   Examples:

   ```ds
   type IntOrBool = int32 | bool
   // layout: { tag: u8, value: int64 }

   type IntOrNull = int32 | null
   // layout: { tag: u8, value: int32 }

   type PointOrLine = Point | Line
   // layout: { tag: u8, data: [u8; max(sizeof)] }
   ```

3. **Boxed** - Large or heterogeneous unions, or when runtime discrimination
   needs RTTI but the variants are not tagged.
   Examples:

   ```ds
   type Dynamic = unknown
   // layout: { typeTag: TypeTag, payload: word }

   type LargeUnion = LargeA | LargeB
   // layout: { tag: u8, data: pointer to variant }
   ```

The inline size threshold is fixed per target for ABI stability.
**Owned unions** (`^(A | B)`) prefer inline representation when the variant is known at runtime
without additional RTTI. If RTTI is required for drop, the union is boxed with an explicit tag.
The `typeTag` in boxed unions points at the RTTI descriptor.

### Dynamic Types (unknown)

**Native targets do not support `any`.** Only `unknown` is available, requiring explicit type checks via RTTI before use. This matches Rust's approach: dynamic typing requires explicit casts and runtime checks.

```ds
const value: unknown = getUnknownValue();
value.foo()              // ERROR: cannot access property on unknown
if (value is User) {
    value.name           // OK: narrowed to User
}
```

`unknown` uses a fat-pointer layout:

```ds
struct unknown {
    typeTag: TypeTag
    payload: word
}
```

The `payload` is a pointer-sized word interpreted by `typeTag`.
`word` is a pointer-sized integer type (u64 on 64-bit, u32 on 32-bit).
Managed references store the object pointer in `payload`.
Small primitives store their bitwise representation directly in `payload`.
Large values are boxed into managed memory and referenced by `payload`.

**JS targets:** Both `any` and `unknown` are supported with standard TypeScript semantics.
`any` bypasses type checking; `unknown` requires narrowing. For portable code, prefer `unknown`.

### Reflection

Destack's types-as-values feature makes `Type<T>` a first-class value, enabling
both compile-time and runtime reflection (as needed).

**Source-level API** (from `@destack-sh/core/reflection`):
```ds
Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct"
    name: string
    id: TypeId              // stable identifier: "@destack-sh/ui/components/button:Button"
    properties: Property[]
    decorators: DecoratorInfo[]
}
```

**Comptime:** Full type information is available as compile-time data.
Type operations execute in the VM interpreter (which runs MIR), so comptime and runtime share the same representation.
This simplifies the design: there's no separate "comptime type format" vs "runtime type format".

```ds
const PROP_COUNT = comptime User.properties.length;    // → literal 3
const HAS_NAME = comptime User.properties.some(p => p.name == "name");  // → true

if (comptime User.properties.some(p => p.type == string)) {
    // branch selected at compile time, other eliminated
}
```

**Runtime:** When type information is needed at runtime, we generate RTTI.
The RTTI table is a static array embedded in the binary.
The native RTTI representation is a compact binary format that maps to the high-level
`Type<T>` API from `language/builtin/core/reflect/type.ds`:

```ds
// high level API
newtype Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct" = "struct"
    name: string
    id: TypeId                  // "myapp/models:User"
    properties: readonly Property[]
    decorators: readonly DecoratorInfo[]
    description?: string
}

// native RTTI: binary format, used by runtime
struct TypeDescriptor {
    id: uint32                  // index into RTTI table
    typeIdOffset: uint32        // offset to TypeId string ("myapp/models:User")
    nameOffset: uint32          // offset to name string ("User")
    size: uint32                // sizeof(T) in bytes
    ...
}
```

At runtime, when user code accesses `User.properties` or `typeOf(value)`, the `TypeDescriptor` data is accessed directly.
(Comptime and runtime share the same MIR representation, so no synthesis or conversion step is needed.)
Runtime `Type<T>` values are represented as `TypeTag` handles that point to `TypeDescriptor` records.
When a vtable exists, slot 0 stores the `TypeTag` handle.
Interface and `unknown` values carry it in fat pointers, and thin pointers recover it via GC metadata when needed.

**Lowering Type<T> operations:**

| Source | Comptime | Runtime (if needed) |
|--------|----------|---------------------|
| `User` (in type position) | Type check | N/A |
| `User` (in value position) | Constant TypeTag | Load from RTTI table |
| `User.name` | Constant "User" | `rtti[user_id].name` |
| `User.properties` | Constant array | Load property descriptors |
| `value instanceof User` | Eliminated if type known | Compare `value.typeTag == @User_TypeTag` |
| `User.is(value)` | Eliminated if type known | Compare `value.typeTag == @User_TypeTag` |
| `typeOf(value)` | Constant if type known | Load `value.typeTag` |

When a value is a thin pointer without an embedded type tag, we get the `TypeTag` handle from GC metadata for comparison.

**RTTI generation rules:**
RTTI (TypeDescriptor records) are only emitted for types that need runtime type checks.
Lower conservatively emits RTTI for any type that might need it:
- Types used with `instanceof` or `T.is` on values of unknown concrete type
- Types used with `typeOf()` on values of unknown concrete type
- Types stored in `unknown` (need RTTI for later extraction)
- Types with runtime reflection (non-comptime `.properties`, `.name`, etc.)
- Types used in untagged unions that require runtime discrimination

Lower does NOT emit RTTI for:
- Types only used with statically-known concrete types
- Types where all `instanceof`/`T.is` checks are eliminated by type narrowing
- Primitives (handled by tag bits, not full TypeDescriptor)

Dead code elimination in the Optimize phase removes unused RTTI entries.
If all type operations resolve at comptime, no RTTI overhead appears in the binary.

## Dispatch

Method calls are resolved and dispatched differently based on structural or nominal types.
TypeScript's duck typing means any object with matching methods can satisfy an interface, which creates some interesting challenges for native codegen.

Polymorphic class dispatch uses vtables stored in the object layout, like C++ and Java.
Interface dispatch uses itabs carried by fat pointers, like Go.
Union dispatch generates type checking code when a value could be multiple types.

### VTable Layout

Classes with virtual methods have a vtable.
VTables are only emitted when virtual dispatch remains after devirtualization.
The vtable is an array of slots with a fixed prefix and method targets.

**VTable structure (conceptual):**
```ds
struct VTable {
    typeTag: TypeTag;    // for instanceof, T.is, and typeOf
    drop: () => void;             // drop glue
    methods: ((...args: unknown[]) => unknown)[]; // virtual method pointers
}
```

**Example vtable layout:**
<pre>
Node vtable:
  slot 0: typeTag = @Node_TypeTag
  slot 1: drop = Node_drop
  slot 2: update = Node.update

Sprite vtable (inherits Node):
  slot 0: typeTag = @Sprite_TypeTag
  slot 1: drop = Sprite_drop
  slot 2: update = Sprite.update      // overrides Node::update
</pre>

**Slot assignment (inheritance-preserving):**
- Slot 0: always `typeTag` (for `instanceof`, `T.is`, `typeOf`)
- Slot 1: always `drop` (drop glue)
- Slots 2+: virtual methods in declaration order
- Child classes inherit all parent slots at the same indices
- Overriding methods reuse the parent's slot index
- New methods append after the last inherited slot

This inheritance-preserving order ensures:
- Upcasting requires no vtable adjustment (same slots, same indices)
- Parent code works on child objects without recompilation
- Binary compatibility when adding methods to leaf classes

**Virtual call lowering:**
```mir
; node.update(delta) where node could be Node or Sprite
v1 = field.get v0, 0           ; load vtable pointer from object
v2 = field.get v1, 2           ; load update method (slot 2)
v3 = call.indirect v2(v0, delta) -> fn(ref<raw void>, i32) -> void ; call with self as first arg
```

**Super calls:**
Super calls compile to direct calls to parent implementation.

```ds
class Sprite extends Node {
    update(delta: float): void {
        super.update(delta)  // call Node.update
        this.animate(delta)
    }
}
```

Lowers to:
```mir
type @Sprite = struct { ref<raw void>, ref<string>, ref<Texture> }

function @Sprite.update(v0: ref<@Sprite>, delta: f32) -> void {
block0(v0: ref<@Sprite>, delta: f32):
    call @Node.update(v0, delta)   ; direct call, no vtable lookup
    call @Sprite.animate(v0, delta)
    return
}
```

### Interface Dispatch (ITabs)

Interfaces use itabs, and interface values carry a fat pointer:
```ds
type InterfaceRef<I> = { objectPtr: &Object; itabPtr: &InterfaceItab<I>; };
```

Each (Type, Interface) pair has its own itab mapping interface fields and methods to concrete layouts.
Itabs are generated for both struct and class implementations.
Static members never appear in vtables or itabs.

#### Structural Interfaces

Structural interfaces require **fat pointers** because the itab layout varies per (Type, Interface) pair:

```ds
interface Drawable { color: uint32; draw(): void; }
interface Resizable { resize(w: int, h: int): void; }

struct Circle { color: uint32; radius: float; }
struct Rectangle { color: uint32; width: float; height: float; }
```

When a `Circle` is used as `Drawable`, we create a fat pointer:

```ds
// fat pointer representation
struct InterfaceRef<I> {
    objectPtr: &unknown;       // actual object (type erased)
    itabPtr: &InterfaceItab<I>;  // interface itab
}
```

**Fat pointer size:** Interface references are exactly `2 * sizeof(usize)` (16 bytes on 64-bit).
The layout is `(objectPtr, itabPtr)` with no padding.
This matches Go's interface representation.

Each (Type, Interface) pair generates its own itab:

```ds
// circle as Drawable
const Circle_Drawable_itab: InterfaceItab<Drawable> = {
    typeTag: @Circle_TypeTag,
    color: 0,
    draw: @Circle.draw,
};

// rectangle as Drawable
const Rectangle_Drawable_itab: InterfaceItab<Drawable> = {
    typeTag: @Rectangle_TypeTag,
    color: 0,
    draw: @Rectangle.draw,
};
```

**Interface call lowering:**

Each (Type, Interface) pair gets its own itab with slots assigned in interface declaration order.
Slot 0 is always `typeTag`, then fields and methods follow in the interface member order.
Interface inheritance flattens base interfaces in extends list order before local members.
Members inherited with the same name and signature reuse the first slot.
Fields reuse slots only when their declared types match.
Conflicting member signatures are errors during analysis.
Field entries store byte offsets, and method entries store function pointers.
The compiler generates the itab at compile time, and interface references carry a pointer to the appropriate itab.
Interface to interface casts rebuild the fat pointer.
The object pointer is preserved and the source itab provides the concrete type tag.
The target itab is resolved from `(typeTag, target interface)` and stored in the new interface reference.

```ds
function render(d: Drawable) { d.draw(); }
```

Lowers to:
```mir
type @Drawable = struct { ref<raw void>, ref<raw void> }

function @render(v0: ref<raw @Drawable>) -> void {
block0(v0: ref<raw @Drawable>):
    ; v0 is a fat pointer: { objectPtr, itabPtr }
    v1 = field.get v0, 0       ; load objectPtr
    v2 = field.get v0, 1       ; load itabPtr
    v3 = field.get v2, 1       ; load draw method from itab slot 1
    call.indirect v3(v1) -> fn(ref<raw void>) -> void ; call with object as self
    return
}
```

Interface property access uses the same itab slots.
Lowering loads the field offset from the itab and applies it to the object pointer.

#### Nominal Interfaces

Nominal interfaces (`newtype interface`) use the same fat pointer representation as structural interfaces.
The difference is typing: nominal interfaces require explicit `implements` declarations.
This makes itabs fully known at compile time and avoids runtime method-set checks.

**Multiple interface implementation:** When a type implements multiple interfaces, each (Type, Interface)
pair gets its own itab. Methods are listed in that interface's declaration order.

**Method ambiguity resolution:** If two interfaces require methods with the same name and signature,
one implementation satisfies both. If signatures differ (same name, different types), the compiler
emits a compile error with guidance on disambiguation:

```ds
interface Reader { read(): bytes }
interface JsonReader { read(): JsonValue }  // different return type

// ERROR: Ambiguous implementation of 'read' for FileParser
// Hint: Use explicit interface qualification or rename one method
class FileParser implements Reader, JsonReader { ... }
```

Resolution options:
1. Rename one method in the interface (if you control it)
2. Use wrapper types with explicit delegation
3. Implement only one interface directly, delegate the other

```ds
newtype interface Hashable {
    hash(): uint64
}

extension for Circle implements Hashable {
    hash(): uint64 { ... }
}
```

#### ITab Generation

For each (Type, Interface) pair where the type implements the interface:

1. Create a static itab with field offsets and method pointers in interface declaration order
2. Store the itab as a global constant
3. When creating an interface reference, pair the object with the appropriate itab
4. For `unknown` or dynamic casts, build and cache the itab at runtime on first use
5. The cache is global per runtime and keyed by `(concrete TypeTag, interface TypeTag)`

**Itab layout:**
```ds
struct InterfaceItab<I> {
    typeTag: TypeTag;  // for T.is on interface refs
    slots: [InterfaceSlot]; // field offsets and method pointers in declaration order
}
```

#### Cost Model

| Operation | Structural Interface | Nominal Interface | Class Virtual |
|-----------|---------------------|-------------------|---------------|
| Reference size | 2 pointers (fat) | 2 pointers (fat) | 1 pointer |
| Method call | 2 loads + indirect | 2 loads + indirect | 2 loads + indirect |
| Creation | Method-set check + itab | Direct itab | Just cast |
| Memory per type | itab per interface | itab per interface | vtable in object |

Structural interfaces enable TypeScript's duck typing but have overhead:
- 2× pointer size for interface references (fat pointer)
- One itab per (Type, Interface) pair
- Cache locality may suffer from double indirection

### Extension Methods

Extension methods are direct calls with the receiver as the first argument.
No vtable or itab lookup is needed.

```ds
extension for Vector2 {
    magnitude(): float { sqrt(this.x * this.x + this.y * this.y) }
}

const v = Vector2 { x: 3, y: 4 };
v.magnitude();  // direct call
```

Lowers to:
```mir
function @Vector2_ext.magnitude(v0: ref<@Vector2>) -> f64 {
block0(v0: ref<@Vector2>):
    v1 = field.get v0, 0       ; load x
    v2 = field.get v0, 1       ; load y
    v3 = fmul v1, v1
    v4 = fmul v2, v2
    v5 = fadd v3, v4
    v6 = intrinsic.sqrt(v5)
    return v6
}

; call site: v.magnitude()
v1 = call @Vector2_ext.magnitude(v0)
```

Extension methods are resolved statically at compile time based on the receiver type.
They do not participate in virtual dispatch.

### Getters and Setters

Property accessors lower to method calls.
There is nothing special about them at the MIR level.

```ds
class Circle {
    #radius: float64

    get area(): float64 { 3.14159 * this.#radius * this.#radius }
    set radius(r: float64) { this.#radius = r }
}

c.area          // call @Circle.get_area(c)
c.radius = 5    // call @Circle.set_radius(c, 5)
```

#### Pattern Matching with Getters

When pattern matching on an object with getters:
- Getter is called **once** per pattern
- Result is bound to the pattern variable
- Evaluation order: left-to-right in pattern

```ds
match (obj) {
    { foo: 0 } => ...  // calls obj.foo getter, compares to 0
    { foo: x } => ...  // calls obj.foo getter, binds to x
}
```

Equivalent to:
```ds
const __foo = obj.foo;  // one getter call
if (__foo === 0) { ... }
else { const x = __foo; ... }
```

## Memory Model

Memory allocation and ownership at the MIR level.
TypeScript/JavaScript uses garbage collection with no explicit memory management.
Destack preserves this simplicity by default (GC managed heap allocation), but enables opt in control for performance critical code.
The **GC** implementation is assumed abstractly as "managed allocate" and TS compatible, we assume potential pauses and add some barriers.

### Allocation Modes

Destack supports three allocation modes, controllable via `AllocationMode` per function.
Users can annotate functions with `@noManaged` or `@stackOnly` decorators to enforce these modes:

| Mode | Allocations Allowed | Use Case |
|------|---------------------|----------|
| `Any` | All | Default, full TS compatibility |
| `NoManaged` | `RawAlloc`, `StackAlloc` | Realtime-safe, no GC pauses |
| `StackOnly` | `StackAlloc` | Embedded, deterministic |

#### Managed Allocation (GC)

`ManagedAlloc` creates GC-tracked objects.
The runtime provides garbage collection; Lower just emits the allocation instructions.
Allocator selection for native targets is configured per-target (`allocator`).

```mir
type @Point = struct { f32, f32 }

function @alloc_example() -> ref<managed @Point> {
block0:
    v0 = managed.alloc @Point -> ref<managed @Point>
    return v0
}
```

**Write barriers:** Lower automatically inserts `Intrinsic::GcWriteBarrier` for all `ManagedReference` field writes.
The runtime uses this for concurrent marking.
See [INTRINSICS.md](INTRINSICS.md#garbage-collection) for details.

**Roots:** Each function has a stack map describing which slots contain managed references.
The GC uses these to find roots during collection.
Managed allocations do not include per object headers.
The allocator side tables store mark bits, size class, and the TypeTag handle used for scanning.

GC implementation details are target-specific and live in the runtime/codegen layers.
The general approach (when GC is enabled) is Go-like: insertion write barriers with a concurrent mark phase.

**Target-dependent behavior:**
GC features are conditional on target configuration. 
For targets without GC (freestanding, `@noManaged` code):
- No write barriers are emitted
- No stack maps are generated
- Managed allocations are a compile error
- Only `^T` ownership and `&T` borrows are available

Lower queries the target profile to determine which GC features to emit.
The runtime provides the actual GC implementation; Lower just emits the hooks.

#### Raw Allocation

`raw.alloc` creates manually-managed heap memory for owned values (`^T`).
Cleanup uses `raw.drop` (with dispose) or `raw.free` (without dispose).

```mir
type @SomeType = struct { i64 }

v0 = raw.alloc @SomeType -> ref<raw @SomeType>
; ... use v0 ...
raw.drop v0    ; drop glue: dispose + deallocate
```

For manual deallocation without dispose (FFI, low-level code):

```mir
v0 = raw.alloc @SomeType -> ref<raw @SomeType>
; ... use v0 ...
raw.free v0    ; just deallocate, no dispose
```

No GC overhead.
Used with ownership annotations (`^T`) for Rust-like semantics.
In debug builds, a tracing allocator (like Zig's) can detect leaks, double-frees, and use-after-free.

#### Stack Allocation

`stack.alloc` creates frame-local storage.
Cleanup uses `stack.drop` for dispose; deallocation happens automatically when the frame exits.

```mir
type @SomeType = struct { i64 }

v0 = stack.alloc @SomeType -> ref<raw addrspace(stack) @SomeType>
; ... use v0 ...
stack.drop v0    ; drop glue: dispose only, frame handles memory
```

No heap allocation.
The optimizer promotes `raw.alloc` to `stack.alloc` via escape analysis when the value doesn't escape the function.

### Drop Glue

The drop instructions (`raw.drop`, `stack.drop`) perform **drop glue**:

1. **Drop owned fields** in reverse declaration order (LIFO, like Rust/C++)
2. **Call dispose** (`Symbol.dispose`) if the type implements `Drop`
3. **Deallocate** (only for `raw.drop`; `stack.drop` skips this)

**Field drop order:** Fields are dropped in reverse declaration order.
This matches C++ and Rust destruction semantics: last declared, first destroyed.
For inherited classes, child fields drop before parent fields.

```ds
class Resource {
    a: ^FileHandle;  // declared first, dropped last
    b: ^Connection;  // declared second, dropped second
    c: ^Buffer;      // declared last, dropped first
}
// drop order: c, b, a
```

Drop glue metadata is attached to MIR type definitions during lowering.
Lower has full DIR type information (including `Drop` trait bounds) and generates the appropriate drop glue for each type.

**Important:** Lower only *marks* ownership on types and values (via `^T` modifiers and allocation instructions).
The actual drop instruction insertion happens in Optimize's `drop-insert` pass, which runs as part of the Verify phase.
This separation ensures drops are placed at precise last use points after all control flow is lowered.

| Instruction | Drop Fields | Call Dispose | Deallocate |
|-------------|-------------|--------------|------------|
| `raw.drop` | Yes (LIFO) | If `Drop` | Yes |
| `stack.drop` | Yes (LIFO) | If `Drop` | No (frame) |
| `raw.free` | No | No | Yes |

For managed allocations (`managed.alloc`), there is no drop instruction.
The GC handles cleanup, with finalizers for any `^T` fields (nondeterministic).

### Ownership

Destack aims to cover the "managedness" spectrum from TS to Go to Rust: implicit GC by default, explicit ownership when needed.
Most code just uses the default, and that should still be plenty fast thanks to real AOT compilation and fixed layouts (more like Go, Java, C#).
Performance critical code adds these ownership modifiers for manual control.

**Explicit Ownership:**

To preserve TypeScript semantics, a plain type `T` always follows the same rules as TypeScript (objects are GC managed references, primitives are values).

| Modifier | Semantics | After `foo(x)` | Who cleans up? |
|----------|-----------|----------------|----------------|
| `T` | GC managed (implicit) | `x` still valid | GC |
| `&T` | Borrow (read only) | `x` still valid | Original owner |
| `&mut T` | Borrow (mutable) | `x` still valid, maybe changed | Original owner |
| `^T` | Owned reference (move-only) | `x` **invalid** | New owner (or GC fallback) |
| `^mut T` | Owned reference (mutable) | `x` **invalid** | New owner (or GC fallback) |

For the default (`T`), the compiler optimizes automatically:
- Small values are passed by copy (registers)
- Large values are GC managed references
- Escape analysis promotes heap to stack when safe

`T` is not owned by anyone, it is implicitly GC managed and freed whenever all references to it are gone.
Many people can hold and mutate `T` as long as they like.
`^T` is for when there should only be one owner.
Accordingly, when calling a function with `^T`, the caller gives up ownership of the value to the callee.
After the transfer, the original binding is invalid:

```ds
function consume(data: ^LargeData) { ... }
const d = LargeData { ... }
consume(^d)    // ownership transferred
print(d.value) // ERROR: use after ownership transfer
```

Use after move is an error.
When a `^T` value reaches its **last proven use** without being transferred, it is **dropped**:

```ds
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    // data can be dropped before the next statement
}
```

`Drop` is a marker interface that opts a type into last use cleanup when possible.
(Types that implement `Drop` must also implement `Symbol.dispose`, which is invoked by the drop glue).
Optimize's `drop-insert` pass inserts drops at last use points (non lexical), including before
control flow merges and before coroutine suspension when the value is not used after resume.

#### Borrowing

`&T` and `&mut T` are explicit references (pointers) to data.
They lower directly to pointer types in MIR:

```ds
function process(data: &Point) { ... }   // read only reference
function mutate(data: &mut Point) { ... } // mutable reference
```

Lowers to (conceptual):
```mir
type @Point = struct { f32, f32 }

function @process(v0: ref<borrowed @Point>) -> void { ... }
function @mutate(v0: ref<borrowed mut @Point>) -> void { ... }
```

Lower treats locals, `this`, globals, and member or index access as addressable places for `&expr`.
Non addressable expressions are materialized into a temporary local before `local.addr` is emitted.
If the borrow target is a member or index on a reference-like base, Lower uses the base value directly and avoids a spill.
Borrowing subfields lowers to explicit address projections (`field.addr`, `element.addr`).
Borrowed references are verified by the borrow check pass in Optimize.
Borrows are created by `field.addr`, `element.addr`, and by calls that return borrowed references with lifetimes.
A borrow ends when the reference value is no longer live.
Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
Dropping or freeing a value while it is borrowed is always an error.
In strict mode, conflicting borrows and invalidating stores are errors.
In lenient mode, the same situations produce warnings.

#### Raw pointers

`*T` and `*mut T` are unsafe pointers with no borrow tracking.
They lower directly to `ref<raw T>` and `ref<raw mut T>`.
Deref and mutation use explicit `load`/`store` and pointer operations.
Conversions between borrowed references and raw pointers are explicit.

#### Address spaces

Lower preserves address space annotations on references for native and accelerator targets.
The default address space is `generic`.
Non generic address spaces are only valid for borrowed and raw references.
`constant` references are always immutable.
Address space changes are explicit and use the `addrspace.cast` intrinsic.

#### Borrow Modes

By default, `&T` and `&mut T` are hints and violations produce warnings.
They help document APIs, guide drops, and enable limited optimizations.
Set `borrowMode: "strict"` in `dsconfig.json` to enforce exclusive `&mut` borrows.
Strict mode enables stronger `noalias` optimizations and errors on violations.
Strict mode forbids:
- Aliasing `&mut` with any other borrow
- Storing `&mut` inside managed objects
- Holding `&mut` across `await` or generator suspension

### Closures

Functions that capture variables become closure values:

```ds
const x = 10
const f = (y: int) => x + y  // captures x
```

Lowers to a closure struct plus a function pointer to the original function.
The closure struct (MIR-level) captures the environment:
- `x: int64`

The closure value pairs the function pointer with the environment:
- `fnPtr: FunctionPointer`
- `env: ManagedReference<ClosureEnv>`

**Capture semantics:**
- `const` bindings are captured by value (copied into closure struct)
- `let` bindings are captured by reference (pointer to original location)
- `@capture("byMove")` captures bindings by move into the environment
- This matches JavaScript's closure semantics

**Environment layout:**
Captured variables are stored in the closure struct in declaration order (order of first capture).
The struct is alignment-packed to minimize size.
Interior pointers are used for reference captures.

The closure body reads its environment via `function.env`.
Closure calls load `fnPtr` and `env`, then call with `call.indirect` and `env=`.
Direct calls to the original function do not carry an environment.

### Copy Elision and Move Semantics

Avoiding unnecessary copies is critical for "systems-level" performance.
Lower implements several strategies to minimize data movement.

**Return Value Optimization (RVO):**
When a function returns a locally-constructed value, we allocate directly into the caller's destination.

**Named RVO (NRVO):**
Extends RVO to named variables when there's a single return path.

**Inline hints:**
The `@inline` decorator is a hint, not a directive. Lower emits the function normally.
The Optimize phase decides whether to actually inline based on:
- Function size and complexity
- Call site frequency (from profiling if available)
- Whether inlining enables further optimizations
`@inline("always")` is a stronger hint but still not guaranteed.
`@inline("never")` prevents inlining (useful for debugging, code size).

**Move Semantics:**
By default, Destack uses TypeScript semantics: objects are GC-managed references.
Assignment shares references; variables remain valid after being passed to functions.
Move semantics only apply with explicit `^T` value types.

### Stack Safety

#### Stack Overflow

Native targets use **guard pages** for stack overflow detection.
Overflow triggers immediate abort with diagnostic (like Go/Rust).

Stack overflow is treated as a bug (infinite recursion), not a recoverable condition.
No stack check prologue overhead in normal functions.

#### Stack Size

Default stack size is target-dependent (typically 1-8 MB).
Configurable via target profile or runtime initialization.

## Control Flow

Control flow constructs (errors, async, generators) lower to MIR blocks and terminators.
JavaScript's `throw`/`catch` and `async`/`await` are powerful but have runtime costs: exception tables, stack unwinding, state machine overhead.
**Result first error handling**: recoverable errors use `Result<T, E>` (zero cost early returns), while `throw` becomes an abort in native code.
Explicit errors in the type system, panics for bugs only (like Rust).
Async functions lower to state machines, preserving JS `Promise` semantics without the JS runtime overhead.

### Errors and Exceptions

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while `throw` is reserved for unrecoverable panics (bugs).

| Mechanism | Use For | Example |
|-----------|---------|---------|
| `Result<T, E>` | Recoverable errors | Parse failures, file not found, network timeout |
| `throw` | Unrecoverable panics | Assertion failures, invariant violations, bugs |

**Panics indicate bugs**, not expected error conditions.
Use `Result` for anything the caller might want to handle.

The `?` operator propagates errors ergonomically:

```ds
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?;    // propagates Err
    const json = parseJson(text)?;            // propagates Err
    Result.ok(Config.from(json))
}
```

Lowers to early return on error.

**Panic (throw):**
`throw` indicates an unrecoverable error (bug, invariant violation).
Unlike traditional exceptions, panics are not meant to be caught.
On native targets, `throw` aborts without unwinding.
Panic policy is configured per target (`panic`), and unwind requires runtime support.

### Coroutines: Async & Generators

TypeScript already has "function coloring": `await` is only valid inside `async function`, `yield` only inside `function*`.
This is baked into the language semantics we preserve.
State machine transformation is therefore the natural implementation strategy.

**Design principle:** Promise is a library type, state machines are a Lower transformation, the runtime glues them together.
External effects cross the runtime boundary through yield and resume so they can be recorded and replayed.

<pre>
┌─────────────────────────────────────────────────────────────────────────┐
│                              Architecture                                │
│                                                                          │
│   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
│   │  async function │───▶│  State Machine  │───▶│     Promise     │     │
│   │    (source)     │    │  (Lower output) │    │ (library type)  │     │
│   └─────────────────┘    └─────────────────┘    └─────────────────┘     │
│                                   │                      ▲              │
│                                   │                      │              │
│                                   ▼                      │              │
│                          ┌─────────────────┐             │              │
│                          │     Runtime     │─────────────┘              │
│                          │  (event loop)   │                            │
│                          └─────────────────┘                            │
└─────────────────────────────────────────────────────────────────────────┘
</pre>

#### Separation of Concerns

| Component | Responsibility | Location |
|-----------|----------------|----------|
| **Lower** | Transform async bodies to state machines, emit `yield` terminators | `compiler/src/lower/` |
| **Async library** | Store state, manage continuations, expose async APIs | `builtin/lib/native/` |
| **Runtime** | Schedule async work and I/O primitives | `builtin/lib/native/` |

Lower does not know async library internals.
Lower emits `yield` terminators, and the runtime hooks them to continuations.

#### State Machine Transformation

Lower transforms each async function into a **tagged union state machine**.
The state is represented as an explicit enum, and all locals that survive across await points
are stored in the state machine struct.

**Components:**
1. **State machine struct** - captures locals that live across await points
2. **Entry function** - creates Promise, starts execution, returns Promise
3. **Poll function** - advances state machine, called on each resume

**State representation:**
The state field is a `uint8` (or larger if needed) representing which await point we're at:
- State 0: initial entry, before first await
- State N: suspended at await point N, waiting for promise
- State MAX: completed (terminal state)

**Local storage strategy:**
All locals that are potentially live across ANY await point are stored in the state struct.
This is conservative but correct. The optimizer may later prove some locals don't need storage.
Locals that are only used between await points are stack-allocated in the poll function.

```ds
// source
async function fetchUser(id: string): Promise<User> {
    const response = await fetch(`/users/${id}`);
    const data = await response.json();
    return User.from(data);
}
```

Conceptually, Lower generates:

```ds
// state machine struct: locals that survive across awaits
struct @fetchUser_StateMachine {
    state: uint8;
    id: string;
    response: Response | null;
    data: JsonValue | null;
    promise: Promise<User>;  // the outer promise to resolve
}

// entry: create state machine and promise, start execution
function fetchUser(id: string): Promise<User> {
    const promise = Promise<User>.new();
    const sm = @fetchUser_StateMachine {
        state: 0,
        id: id,
        response: null,
        data: null,
        promise: promise
    };
    @fetchUser_poll(sm);  // start execution
    return promise;        // return immediately
}

// poll: advance state machine one step
function @fetchUser_poll(sm: &@fetchUser_StateMachine): void {
    match (sm.state) {
        0 => {
            const p = fetch(`/users/${sm.id}`);
            p.then(response => {
                sm.response = response;
                sm.state = 1;
                @fetchUser_poll(sm);
            });
        }
        1 => {
            const p = sm.response!.json();
            p.then(data => {
                sm.data = data;
                sm.state = 2;
                @fetchUser_poll(sm);
            });
        }
        2 => {
            sm.promise.resolve(User.from(sm.data!));
        }
    }
}
```

#### Yield Terminator

At the MIR level, `await` becomes a `yield` terminator:

```mir
yield <awaited_promise>, <resume_block>(<captured_values>)
```

Semantics:
1. **Suspend**: Save current state, return control to caller
2. **Register**: Runtime calls `awaited_promise.then(resume_callback)`
3. **Resume**: When promise resolves, runtime calls resume block with result

Captured values are passed as resume arguments, with the resolved value appended after them in the
resume block parameter list.

#### Generators

Generators use the same state machine approach but yield values to caller instead of awaiting:

```ds
function* range(start: int, end: int): Generator<int> {
    for (let i = start; i < end; i++) {
        yield i;
    }
}
```

The state machine implements `Iterator<T>`:
- `next()` advances to next yield, returns `{ value: T, done: boolean }`
- State persists across `next()` calls

#### Async Generators

Async generators combine both: `await` suspends, `yield` produces values.

```ds
async function* fetchPages(urls: string[]): AsyncGenerator<Page> {
    for (const url of urls) {
        const response = await fetch(url);
        yield await response.json();
    }
}
```

`next()` returns `Promise<IteratorResult<T>>`:
- Caller awaits the returned Promise
- State machine may suspend multiple times per `next()` call (on awaits)
- Eventually yields a value or completes

## Concurrency

Destack preserves JS and TS concurrency semantics by default while enabling native level parallelism on supported targets.
The core ideas are a single threaded event loop by default, `Promise` and `async` for concurrency, and `Worker` for parallelism.

#### Threading Model
Default behavior is a single threaded event loop with microtask and macrotask queues.
Native targets support worker threads with the same `Worker` API.
JS targets map to real JS `Worker` instances.
WASM targets map to host specific workers when available.
Native targets map to OS threads with message passing.
Shared memory is explicit and opt-in.

#### Memory Model
Shared memory follows JS Atomics semantics.
Atomic operations require explicit ordering and scope metadata.
There are no implicit defaults for atomic ordering in Destack.
Non atomic loads and stores have no cross thread ordering guarantees.
Data races on shared non atomic memory are undefined behavior on native targets.
This preserves JS and TS semantics while enabling native performance when code uses atomics.

#### GC and Threads
GC heaps are per worker by default to match JS semantics and avoid sharing mutable GC objects.
GC managed objects are not shared across workers unless explicitly frozen or copied.
Shared memory uses raw pointers or explicit shared buffers.
Native targets may add a shared heap mode when the runtime provides the required GC and synchronization support.

---

# Target Configuration

Lower behavior is configured by target policies defined in `dsconfig.json` and the target profile.
See `language/workspace/src/config/target.rs` for the canonical definitions.

## Runtime Checks

### Bounds Checks

Control array and slice bounds checking.

| Variant | Behavior |
|---------|----------|
| `Always` | Bounds checks in all builds |
| `Debug` | Bounds checks only in debug builds (default) |
| `Never` | No bounds checks (unsafe, fastest) |

Bounds check failure triggers a panic (abort on native targets).

### Overflow Checks

Control integer overflow checking.

| Variant | Behavior |
|---------|----------|
| `Always` | Overflow checks in all builds |
| `Debug` | Overflow checks only in debug builds (default) |
| `Never` | No overflow checks; signed overflow is UB |

Overflow check failure triggers a panic.
Explicit wrapping (`+%`) and saturating (`+|`) operators bypass this policy.

## Error Handling

### Panic Policy

What happens when a panic occurs (via `throw` or failed assertions).

| Variant | Behavior |
|---------|----------|
| `Abort` | Terminate immediately via `Intrinsic::Abort` (default) |
| `Unwind` | Stack unwinding for destructors |

Abort is simpler and has no overhead.
Unwind enables deterministic destructor calls but requires exception tables.

### Unwind Format

Format for unwind information (for debuggers and profilers).

| Variant | Platform |
|---------|----------|
| `None` | No unwind info |
| `Dwarf` | DWARF CFI (Unix, macOS) |
| `Seh` | Structured Exception Handling (Windows) |

Even with `panic: Abort`, minimal unwind info can be emitted for debuggers.

## Debug and Symbols

### Debug Info Level

Granularity of debug information.

| Variant | Content |
|---------|---------|
| `None` | No debug info |
| `Line` | Line numbers and function names |
| `Full` | Line numbers, variables, types (default for debug) |

Debug info is emitted as DWARF (or platform equivalent) by codegen.

### Debug Execution Mode

Selects how debug workflows execute native or VM code.

| Variant | Behavior |
|---------|----------|
| `Auto` | Use `Deopt` for debug builds, `Native` for release |
| `Vm` | Force interpreter execution |
| `Deopt` | Run native with deopt-first debugging |
| `Native` | Run native only (no deopt) |

`debugMode` controls whether Lower emits full deopt metadata and inline frame maps.
`debugInfo` only affects native symbol/line metadata, not deopt fidelity.

### Strip Level

Symbol table stripping for release builds.

| Variant | Behavior |
|---------|----------|
| `None` | Keep all symbols |
| `Partial` | Strip internal symbols, keep exports |
| `Full` | Strip all symbols (smallest binary) |

## Execution Policy

Execution policy governs tiering, profiling, and determinism for native targets.
JS/TS targets ignore these settings and rely on their external runtimes.

### Tiering Model

Destack uses two execution tiers:

- **VM**: interpreter for comptime, deterministic debugging, and fallback execution
- **Native**: optimized native code for production performance

There is no baseline native tier by default.
The `debugMode` policy selects VM vs native execution for debugging workflows.

### Profiling Mode

Controls how runtime profiling data is collected for tiering and optimization.

| Variant | Behavior |
|---------|----------|
| `None` | No profiling collection |
| `Counters` | Call/branch/allocation counters only |
| `Sampling` | Sampling only (periodic opcode/site sampling) |
| `Hybrid` | Counters + sampling, optional inline caches (default) |

Profiling is scoped per isolate and consumed by the optimizer and runtime.

### Speculation Policy

Controls guarded speculative optimizations in native code.

| Variant | Behavior |
|---------|----------|
| `None` | No speculative optimizations |
| `Guarded` | Guarded speculations with explicit deopt metadata (default) |
| `Aggressive` | Wider speculation surface with more guards |

All speculations must have a guard, a deopt map, and a recorded deopt reason.
Guard failures are local: in `debugMode = Deopt` they trigger deopt, otherwise they branch to the slow path.
There is no global invalidation for static layouts.

### OSR Policy

Controls on-stack replacement (VM → native) entry placement.

| Variant | Behavior |
|---------|----------|
| `Disabled` | No OSR |
| `LoopHeaders` | OSR at loop headers (default) |
| `Explicit` | OSR only at explicit sites |

Each OSR entry must define live values and phi materialization.

### Safepoint Policy

Controls safepoint insertion for preemption and deopt latency.

| Variant | Behavior |
|---------|----------|
| `CallsAllocBackEdges` | Calls, allocations, loop back-edges only |
| `Budgeted` | Add instruction-budget safepoints |

`safepointInterval` sets the instruction interval when `Budgeted` is enabled.

### Determinism Policy

Controls scheduling and randomness determinism.

| Variant | Behavior |
|---------|----------|
| `BestEffort` | No determinism guarantees |
| `Deterministic` | Deterministic scheduling + controlled randomness (default) |

`Deterministic` fixes scheduler decisions and PRNG seeds.

### Replay Policy

Controls whether external I/O is recorded or replayed.

| Variant | Behavior |
|---------|----------|
| `Off` | External I/O is not recorded |
| `Record` | Record external I/O |
| `Replay` | Replay external I/O |

`Record` records external I/O at runtime boundaries; unshimmed FFI/syscalls are rejected in this mode.
`Replay` consumes recorded external I/O and rejects unlogged effects.

## Memory

### Allocator

Global allocator selection for native targets.

| Variant | Description |
|---------|-------------|
| `System` | Platform default (malloc/free) |
| `MiMalloc` | Microsoft's mimalloc (fast, low fragmentation) |
| `JeMalloc` | FreeBSD's jemalloc (good for large heaps) |
| `Custom` | User-provided allocator |

The allocator provides both managed (GC) and raw allocations.

### Borrow Mode

How borrow annotations are enforced.

| Variant | Behavior |
|---------|----------|
| `Hint` | Warnings only, no hard errors (default) |
| `Strict` | Hard errors on borrow violations; enables `noalias` |

Strict mode enables stronger optimizations but requires more careful code.

## Codegen

### Relocation Model

Position-independent code generation.

| Variant | Use Case |
|---------|----------|
| `Static` | Fixed addresses (executables on some platforms) |
| `Pic` | Position-independent code (shared libraries) |
| `Pie` | Position-independent executable (default for security) |

### Link Mode

Preference for linking dependencies.

| Variant | Behavior |
|---------|----------|
| `Static` | Prefer static linking (larger binary, no runtime deps) |
| `Dynamic` | Prefer dynamic linking (smaller binary, runtime deps) |

### CPU and Features

Target CPU and feature detection.

- `cpu`: Target CPU model (e.g., "haswell", "apple-m1", "generic")
- `cpuFeatures`: Enabled features (e.g., "avx2", "neon", "simd128")

Lower uses `cpuFeatures` to gate SIMD codegen.
If a feature is unavailable, vector operations scalarize to loops.
