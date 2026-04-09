# Lower

Lower turns elaborated, executed, typed DIR into canonical target-conditioned MIR.
This is the phase where semantic DIR becomes executable representation.

## Objectives

The performance target is straightforward:

- Rust-tier performance for explicit ownership and manual control paths
- Go-tier predictability for ordinary managed code
- competitive TypeScript compatibility without speculative runtime warmup

Lower is not an optimizer.
Lower is not a generic metadata repair pass.
Lower's job is to commit to representation.

## Pipeline

Lower receives canonical profile-specific DIR after Analyze, Elaborate, and Execute.
Lower produces canonical MIR with target-conditioned representation decisions.

### Input: Canonical DIR

By the time Lower runs:

- desugaring is complete
- patterns are expanded
- types are inferred
- resolutions are attached
- static arguments are reified
- comptime execution has patched the tree

Lower should not have to rediscover frontend semantics.
It should only have to choose executable representation.

### Output: Target-Conditioned MIR

Lower commits to concrete decisions such as:

- data layout and alignment
- calling convention shape
- managed reference representation
- concrete type and layout realization
- runtime type identity links
- dispatch structures
- policy-controlled checks

## Architecture

The intended Lower model is deliberately small.
The main architectural nouns are:

1. `ModuleLowerer`
2. `FunctionLowerer`
3. `InstanceKey`

`ModuleLowerer` owns module orchestration, declaration state, instance queues, imports, literals, dispatch emission, and final MIR assembly.
`FunctionLowerer` owns body emission.
`InstanceKey` is the architectural home for concrete callable and type instantiation.

`TypeLowerer` remains an important helper owned by `ModuleLowerer`.
`TypeLowerer` answers representation questions:

- what MIR type represents this DIR type
- what the concrete layout is
- what `TypeDescriptor` metadata exists
- what scan and field metadata exist

The top-level flow stays intentionally simple:

1. `declare`
2. `lower`
3. `finish`

`declare` registers roots, globals, literals, imports, and initial callable shells.
`lower` drains the function-body work queue while realizing types and call targets lazily on demand.
`finish` emits final dispatch tables and metadata, then finishes MIR.

Lower is also responsible for the remaining major representation commitments:

- native monomorphization over concrete instances
- `TypeDescriptor` emission and attachment
- scan-shape realization for managed layouts
- exception lowering over MIR exceptional control flow

## Representation Overview

Lower treats layout as a queryable cached graph so it can answer representation questions at any point during lowering.
Most types lower exactly the way their surface semantics suggest.

| Category | Shape | Placement | Notes |
| --- | --- | --- | --- |
| Scalar/Immediate | scalar value | fixed width | includes tagged pointer immediates |
| Pointer | address or handle | pointer-sized | raw address, borrowed address, or managed handle |
| Tuple | ordered fields | packed/offset fields | homogeneous is still tuple |
| Struct | named fields | packed/offset fields | nominal, value semantics |
| Class | instance fields | managed handle + payload layout | reference semantics |
| Array/Slice | element + length | header + data | policy: inline vs heap |
| Function | signature | function pointer or function value | closure values pair fn pointer with env pointer |
| Tagged Union | tag + payload | inline or boxed | tag value + payload layout |
| Untagged Union | set of layouts | external discrimination | type checks or caller-provided discrimination |
| Interface | dispatch surface | itab/vtable + data | separate dispatch layout |
| Intersection | composed view | no new storage | layout = primary + itabs |

### Unions

Union layout is chosen per union:

- **Inline tagged**: tag + payload in one block
- **Boxed tagged**: tag + pointer to payload
- **Untagged**: no tag, relies on explicit runtime type identity or external discrimination

`null` and `undefined` are distinct union elements with distinct tags.
The only special case is a union of a single reference type plus `null`, which lowers to a nullable reference.

### Dispatch

Dispatch layout is modeled separately from data layout.
Vtables are class method tables for virtual dispatch.
Itabs are interface tables for structural or nominal interface dispatch.

A type layout can reference zero or more dispatch layouts, but dispatch layouts never alter data placement.
Vtables are only emitted for classes that still require virtual dispatch after devirtualization.
Interface dispatch always uses itabs, even when the concrete type is a class.

### String Identity

`===` on strings is value equality, not reference equality.

```ds
const a = "hello";
const b = "hel" + "lo";
a === b  // true
```

### Symbol Identity

- `Symbol("desc")` creates a unique symbol each call
- `Symbol.for("key")` returns the same symbol for a given key
- `===` compares symbol identity

```ds
Symbol("a") === Symbol("a")         // false
Symbol.for("a") === Symbol.for("a") // true
```

The `Symbol.for()` registry is global across all modules in a compilation unit.

### Property Enumeration

Object properties enumerate in declaration order.
This applies to `Object.keys()`, `for...in`, and runtime property reflection.

### Default Arguments

Default argument expressions evaluate per call when the argument is `undefined`.
Evaluation occurs in the function's scope.

```ds
function log(timestamp = Date.now()) { ... }
log()  // evaluates Date.now() on this call
log()  // evaluates Date.now() again
```

## Runtime Boundary

Lower does not special-case types like `String` or `Array<T>` as compiler-only concepts.
These are defined in `language/builtin/language/native/` as regular Destack types with builtin or intrinsic support where needed.

## Instantiation and Resolution

### Monomorphization

DIR already carries `Instance` information from analysis, but Lower is still catching up to that model.
The intended native strategy is instance-driven monomorphization:

1. start from root callable instances
2. declare callable shells
3. lower reachable bodies
4. discover new concrete instances while lowering
5. continue until no new instances are discovered

That fixpoint model is the intended shape.
The current implementation only partially realizes it.
`InstanceKey` exists today as the architectural home for this work, but full generic callable instantiation is still incomplete.

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

For JS and TS targets, polymorphism can remain erased where the target supports it.
For native targets, concrete instance lowering is required for layout, dispatch, and calling convention decisions.

### Name Mangling

Monomorphized functions need unique, deterministic names for linking.
We use a human-readable scheme (inspired by Rust and Zig):

**Format:** `@<module_path>.<type>.<method>__<type_args>__h<hash>`

**Examples:**
```mir
@std.collections.Map.get__string__i32__h7f9a3e1     // Map<string, int32>.get()
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
v2 = int.add v0, v1
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
v1 = call User.getName(v0) : fn(User) -> String
```

**Virtual dispatch** (virtual method on class or interface):
```ds
node.update(delta)  // static resolution { target: Node::update }, but virtual
```

Lowers to explicit virtual dispatch:
```mir
call.virtual v0, Node, 2(delta) : fn(ref<Node, managed, readonly>, int32) -> void
```

Lower keeps dispatch as `call.virtual` so optimizer and devirtualization passes can reason about it directly.
Declared and resolved dispatch targets are tracked in MIR dispatch metadata, not in call syntax.
Codegen legalization can later rewrite to direct calls or explicit `call.indirect` sequences.

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
// layout: ref?<T, managed, readonly>
```

All other unions that include `null` or `undefined` lower to tagged union layouts:
```ds
type MaybeInt = int | null
type MaybeRefOrUndefined<T> = T | null | undefined
```

Optional parameters (`x?: T`) lower as `x: T | undefined` and remain explicit in the type system.

### BigInt

BigInt (`bigint` type, `42n` literals) is a **library type with operator overloading**.
It is not a compiler intrinsic; it's defined in `language/builtin/language/native/` and uses the standard operator interfaces (`Add`, `Subtract`, `Multiply`, etc.).

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
| `^string` | owned `str` value | Owned immutable string value |
| `&string` | `&str` | Borrowed immutable view |

Most code uses `string` (GC-managed).
Use `StringBuilder` for performance-critical string construction.

#### String Layout

The immutable `String` header stores only semantic payload metadata and caches.
Growth state belongs to `StringBuilder`, not `String`.
The header layout is identical across targets, and only the payload encoding changes.
UTF-8 payloads use `uint8` data and UTF-16 payloads use `uint16` data.

```ds
struct String {
    lengthUtf16: uint32;      // utf-16 code unit count for ts compatibility
    lengthBytes: uint32;      // byte length of utf-8 data
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
Owned mutable growth lives in `StringBuilder`, which is a separate nominal type from immutable `String`.

The mutable builder form is:

```ds
struct StringBuilder {
    lengthUtf16: uint32;
    lengthBytes: uint32;
    capacity: uint32;
    data: *uint8;
}
```

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
type string = ref<struct readonly { uint32, uint32, uint64, uint32, uint32, *uint8 }>

function template_example(v0: string): string {
bb0(v0: string):
    v1 = global.const str_Hello
    v2 = call string.concat(v1, v0) : fn(string, string) -> string
    v3 = global.const str_Bang
    v4 = call string.concat(v2, v3) : fn(string, string) -> string
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
| `T[]` | `ref<Array<T>, managed, readonly>` | Heap, reference semantics |
| `TypedArray` | `Type::Reference(kind: Raw)` to buffer | Direct memory access |

**Important:** `T[]` is NOT `Array<unknown>`. After monomorphization, we know T.
`Array<unknown>` boxes elements and uses runtime type identity for type checks.

**Array operations:**
```mir
; a[i] where a: int[], a is v0, i is v1
v2 = field.get v0, 2        ; get data pointer
v3 = element.get v2, v1     ; load element at index

; a.push(x) where x is v4
call Array.push(v0, v4) : fn(ref<Array, managed>, Element) -> void

; a.length
v5 = field.get v0, 1        ; load length field
```

**Bounds checks:**
Array and slice indexing emits bounds checks by default.
The policy is configured per target (`boundsChecks` in `destack.json` or via `safetyPreset`):
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
Strings use `StringSlice` for the same role.
They are distinct from borrowing the array object itself.
Functions that accept a slice can take `Slice<T>` or `&Slice<T>`.
Borrowing an array object does not imply a slice view.
Borrowing a `string` object does not imply a `StringSlice` view.

### Structs and Classes

Structs are value types and do not have reference identity.
Classes are reference types and carry identity.
Ownership modifiers (`^T`, `&T`) change storage and lifetime and never change identity semantics.
Lower may represent structs by reference when the observable semantics remain value based.

| Aspect | struct | class |
|--------|--------|-------|
| Reference identity | No (`===` is compile error) | Yes (`===` compares reference identity) |
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

Both lower to nominal instance layouts with computed property offsets. The key difference is **reference identity**: classes have it, structs do not. Both can have **runtime type identity** when needed for `instanceof`, `T.is`, or `typeOf`.

#### Runtime Type Identity

Runtime type identity is unified via `TypeDescriptor` handles, with `TypeId` available as a compact lowered identity token when needed.
Polymorphic classes store a vtable pointer in the object layout for virtual dispatch.
If any class in a lineage requires virtual dispatch, every class in that lineage includes a vtable pointer at offset 0 so upcasts need no pointer adjustment.
Vtable slot 0 stores the `TypeDescriptor` for fast `instanceof`, `T.is`, and `typeOf`.
Structs remain headerless and never store a vtable pointer.
Thin managed references recover `TypeDescriptor` from metadata when needed.
Interface and `unknown` values carry `TypeDescriptor` in fat pointers.
Class references are thin managed references, so the vtable pointer must live in the object layout when the chosen lowering uses in object dispatch metadata.

Metadata lookup only applies to managed references.
Non-managed values require explicit tags, fat pointers, or compile-time type knowledge.

TypeDescriptor records are only emitted when runtime type checks are possible:
- Used with `instanceof`, `T.is`, or `typeOf` on unknown values
- Stored in `unknown`
- Used in runtime reflection
- Used in untagged unions that require runtime discrimination
(Analyze already records a `RuntimeCheckKind` per guard expression for us to select between constant folding, union tags, or type descriptors).

#### Struct Layout

Structs have no **reference identity** (no `===`).
Structs are always headerless and use metadata or fat pointers for runtime type identity.

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

**Prefer discriminated unions** for performance-critical code to avoid runtime type identity lookups:

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

String tags are interned to integers (see [String Equality](#string-equality)).

#### Class Layout

Polymorphic classes have vtable pointers for virtual dispatch and runtime type identity:

```ds
class Node {
    name: string;
    update(delta: float): void { }
}
```

Native layout:
```ds
struct NodeLayout {
    vtablePtr: &Vtable,      // offset 0, vtable[0] = Node_TypeDescriptor
    name: ref<string>,       // offset 8
}
```

Classes have both reference identity (`===` compares managed identity) and type identity (via vtable or metadata).
(This is consistent with Java and C++ class objects while keeping struct layouts headerless like Go.)
Non-polymorphic classes omit the vtable pointer and use metadata or fat pointers for runtime type identity when needed.

Class layouts are static on native targets.
There are no hidden classes or runtime shape transitions.
Dynamic property addition must use explicit map/dictionary types.

#### Managed Object Metadata

Managed objects have no per object GC header.
GC metadata is stored out of line in allocator side tables, similar to Go.
TypeDescriptor values are canonical runtime metadata handles, while `TypeId` is the compact lowered identity token when one is needed for tables or side data.
Null managed references use the null managed handle value.
Undefined is represented through tagged unions, not pointer tagging.

Per span metadata includes:
- Mark bits for GC tracing
- Size class and allocation layout info
- A `LayoutId` per object for scanning
- A `TypeDescriptor` when runtime type queries are enabled

Polymorphic classes store a vtable pointer in the object for virtual dispatch and fast `instanceof`/`T.is`.
Structs remain headerless and rely on metadata or fat pointers for runtime type identity.

Tradeoffs:
- Predictable object layouts and smaller per object overhead
- Thin pointer runtime type identity queries require a metadata lookup

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
   // layout: ref?<string, managed, readonly>
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
   needs runtime type identity but the variants are not tagged.
   Examples:

   ```ds
   type Dynamic = unknown
   // layout: { typeDescriptor: TypeDescriptor, payload: word }

   type LargeUnion = LargeA | LargeB
   // layout: { tag: u8, data: pointer to variant }
   ```

The inline size threshold is fixed per target for ABI stability.
**Owned unions** (`^(A | B)`) prefer inline representation when the variant is known at runtime
without additional runtime type identity. If runtime type identity is required for drop or inspection, the union is boxed with an explicit tag.
The `typeDescriptor` in boxed unions points at the runtime type descriptor.

### Dynamic Types (unknown)

**Native targets do not support `any`.** Only `unknown` is available, requiring explicit type checks via runtime type identity before use. This matches Rust's approach: dynamic typing requires explicit casts and runtime checks.

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
    typeDescriptor: TypeDescriptor
    payload: word
}
```

The `payload` is a pointer-sized word interpreted by `typeDescriptor`.
`word` is a pointer-sized integer type (u64 on 64-bit, u32 on 32-bit).
Managed references store the object pointer in `payload`.
Small primitives store their bitwise representation directly in `payload`.
Large values are boxed into managed memory and referenced by `payload`.

**JS targets:** Both `any` and `unknown` are supported with standard TypeScript semantics.
`any` bypasses type checking; `unknown` requires narrowing. For portable code, prefer `unknown`.

### Reflection

Destack's types-as-values feature makes `Type<T>` a first-class value, enabling
both compile-time and runtime reflection (as needed).

**Source-level API** (from `@destack/core/reflection`):
```ds
Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct"
    name: string
    id: TypeId              // stable identifier: "@destack/ui/components/button:Button"
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

**Runtime:** When type information is needed at runtime, Lower emits `TypeDescriptor` records.
The exact native representation is still an implementation detail, but the semantic contract is stable: runtime type values are `TypeDescriptor` handles that map to the high-level `Type<T>` API from `language/builtin/intrinsic/reflect/type.ds`:

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

// intended native type descriptor shape
struct TypeDescriptor {
    id: uint32                  // index into the type descriptor table
    typeIdOffset: uint32        // offset to TypeId string ("myapp/models:User")
    nameOffset: uint32          // offset to name string ("User")
    size: uint32                // sizeof(T) in bytes
    ...
}
```

At runtime, when user code accesses `User.properties` or `typeOf(value)`, the `TypeDescriptor` records are accessed directly.
(Comptime and runtime share the same MIR representation, so no synthesis or conversion step is needed.)
Runtime `Type<T>` values are represented as `TypeDescriptor` handles.
When a vtable exists, slot 0 stores the `TypeDescriptor` handle.
Interface and `unknown` values carry it in fat pointers, and thin managed references recover it via GC metadata when needed.

**Lowering Type<T> operations:**

| Source | Comptime | Runtime (if needed) |
|--------|----------|---------------------|
| `User` (in type position) | Type check | N/A |
| `User` (in value position) | Constant TypeDescriptor | Load from the type descriptor table |
| `User.name` | Constant "User" | `type_descriptor_table[user_id].name` |
| `User.properties` | Constant array | Load property descriptors |
| `value instanceof User` | Eliminated if type known | Compare `value.typeDescriptor == @User_TypeDescriptor` |
| `User.is(value)` | Eliminated if type known | Compare `value.typeDescriptor == @User_TypeDescriptor` |
| `typeOf(value)` | Constant if type known | Load `value.typeDescriptor` |

When a value is a thin pointer without an embedded type descriptor, we get the `TypeDescriptor` handle from GC metadata for comparison.

**TypeDescriptor emission rules:**
TypeDescriptor records are only emitted for types that need runtime type checks or runtime metadata queries.
Lower conservatively emits TypeDescriptor records for any type that might need them:
- Types used with `instanceof` or `T.is` on values of unknown concrete type
- Types used with `typeOf()` on values of unknown concrete type
- Types stored in `unknown` (need runtime type identity for later extraction)
- Types with runtime reflection (non-comptime `.properties`, `.name`, etc.)
- Types used in untagged unions that require runtime discrimination

Lower does not emit TypeDescriptor records for:
- Types only used with statically-known concrete types
- Types where all `instanceof`/`T.is` checks are eliminated by type narrowing
- Primitives (handled by tag bits, not full TypeDescriptor)

Dead code elimination in Optimize can remove unused runtime type metadata.
If all type operations resolve at comptime, no runtime type identity overhead appears in the final binary.

## Dispatch

Method calls are resolved and dispatched differently based on structural or nominal types.
TypeScript's duck typing means any object with matching methods can satisfy an interface, which creates some interesting challenges for native codegen.

Polymorphic class dispatch uses vtables stored in the object layout, like C++ and Java.
Interface dispatch uses itabs carried by fat pointers, like Go.
Union dispatch generates type checking code when a value could be multiple types.

### Vtable Layout

Classes with virtual methods have a vtable.
VTables are only emitted when virtual dispatch remains after devirtualization.
The vtable is an array of slots with a fixed prefix and method targets.

**Vtable structure (conceptual):**
```ds
struct Vtable {
    typeDescriptor: TypeDescriptor;    // for instanceof, T.is, and typeOf
    drop: () => void;             // drop glue
    methods: ((...args: unknown[]) => unknown)[]; // virtual method pointers
}
```

**Example vtable layout:**
<pre>
Node vtable:
  slot 0: typeDescriptor = @Node_TypeDescriptor
  slot 1: drop = Node_drop
  slot 2: update = Node.update

Sprite vtable (inherits Node):
  slot 0: typeDescriptor = @Sprite_TypeDescriptor
  slot 1: drop = Sprite_drop
  slot 2: update = Sprite.update      // overrides Node::update
</pre>

**Slot assignment (inheritance-preserving):**
- Slot 0: always `typeDescriptor` (for `instanceof`, `T.is`, `typeOf`)
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
call.virtual v0, Node, 2(delta) : fn(ref<Node, managed, readonly>, float32) -> void
```

Lower preserves this as virtual dispatch in MIR.
Late optimization and backend lowering decide whether this stays indirect or devirtualizes.

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
type Sprite = struct { ref<void, raw, readonly>, ref<string>, ref<Texture> }

function Sprite.update(v0: ref<Sprite>, delta: float32): void {
bb0(v0: ref<Sprite>, delta: float32):
    call Node.update(v0, delta) : fn(ref<Sprite>, float32) -> void   ; direct call, no vtable lookup
    call Sprite.animate(v0, delta) : fn(ref<Sprite>, float32) -> void
    return
}
```

### Interface Dispatch (ITabs)

Interfaces use itabs, and interface values carry a fat pointer:
```ds
type InterfaceRef<I> = { objectPtr: &Object; itab: usize };
```

Each (Type, Interface) pair has its own itab mapping interface fields and methods to concrete layouts.
Method slots preserve both identities: the interface method declaration and the concrete target implementation.
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
    itab: usize;               // itab handle (ItabId)
}
```

**Fat pointer size:** Interface references are exactly `2 * sizeof(usize)` (16 bytes on 64-bit).
The layout is `(objectPtr, itab)` with no padding.
This preserves Go's two word footprint, but uses an `ItabId` handle instead of an itab pointer.

Each (Type, Interface) pair generates its own itab metadata entry:

```ds
itab_id Circle_Drawable = 12
itab_id Rectangle_Drawable = 13

itab[12] = { typeDescriptor: @Circle_TypeDescriptor, color: 0, draw: @Circle.draw }
itab[13] = { typeDescriptor: @Rectangle_TypeDescriptor, color: 0, draw: @Rectangle.draw }
```

**Interface call lowering:**

Each (Type, Interface) pair gets its own itab with slots assigned in interface declaration order.
Slot 0 is always `typeDescriptor`, then fields and methods follow in the interface member order.
Interface inheritance flattens base interfaces in extends list order before local members.
Members inherited with the same name and signature reuse the first slot.
Fields reuse slots only when their declared types match.
Conflicting member signatures are errors during analysis.
Field entries store byte offsets, and method entries store function pointers.
The compiler generates the itab metadata at compile time, and interface references carry an itab handle.
Interface to interface casts rebuild the fat pointer.
The object pointer is preserved and the source itab provides the concrete type descriptor.
The target itab is resolved from `(typeDescriptor, target interface)` and stored in the new interface reference.

```ds
function render(d: Drawable) { d.draw(); }
```

Lowers to:
```mir
type Drawable = struct { ref<void, raw, readonly>, usize }

function render(v0: ref<Drawable, raw, readonly>): void {
bb0(v0: ref<Drawable, raw, readonly>):
    call.interface v0, Drawable, 1() : fn(Drawable) -> void
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

1. Create an itab metadata entry with field offsets and method pointers in interface declaration order
2. Assign the entry a deterministic `ItabId`
3. When creating an interface reference, pair the object with the appropriate `ItabId`
4. For `unknown` or dynamic casts, resolve the target `(Type, Interface)` pair and materialize the matching `ItabId`
5. Runtime dispatch reads the `ItabId` and indexes the module itab table

**Itab layout:**
```ds
struct InterfaceItab<I> {
    typeDescriptor: TypeDescriptor;  // for T.is on interface refs
    slots: [InterfaceEntry]; // field offsets and method pointers in declaration order
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
function Vector2_ext.magnitude(v0: ref<Vector2>): float64 {
bb0(v0: ref<Vector2>):
    v1 = field.get v0, 0       ; load x
    v2 = field.get v0, 1       ; load y
    v3 = float.mul v1, v1
    v4 = float.mul v2, v2
    v5 = float.add v3, v4
    v6 = intrinsic.sqrt(v5)
    return v6
}

; call site: v.magnitude()
v1 = call Vector2_ext.magnitude(v0) : fn(ref<Vector2>) -> float64
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
Lower owns the target-specific realization of MIR managed semantics.
This includes the managed reference representation, runtime object layout hooks, stack maps, barrier insertion, and collector integration policy.
The current native or VM managed heap is one backend for that contract, not the definition of `managed` itself.
WasmGC is another valid lowering target for the same MIR `managed` semantics.

```mir
type Point = struct { float32, float32 }

function alloc_example(): ref<Point, managed, readonly> {
bb0:
    v0 = managed.alloc Point -> ref<Point, managed, readonly>
    return v0
}
```

**Write barriers:** Lower automatically inserts `Intrinsic::WriteBarrier` for managed heap edge updates when the target collector requires it.
The runtime uses this for concurrent marking.
See [the MIR intrinsic reference](../../mir/README.md#intrinsics) for details.

**Roots:** Each function has a stack map describing which slots contain managed references.
The GC uses these to find roots during collection.
Managed allocations do not include per object headers.
The allocator side tables store mark bits, size class, and the `LayoutId` used for scanning.
Runtime type queries use `TypeDescriptor` handles rather than collector-local type tags.
The exact scan shape is derived from MIR layout facts rather than from ad hoc collector-local type knowledge.

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
For WasmGC-style targets, Lower may map `managed.alloc`, managed references, and runtime type metadata directly onto the target GC object model instead of the native Destack collector backend.

#### Managed Reference Representation

Managed reference representation is a target and runtime policy, not a separate MIR type.
By default, managed references use pointer width (`usize`) in native layouts.
That machine word may be a direct pointer, compressed handle, object table index, page directory locator, or another equivalent runtime managed representation.
Pointer compression can be enabled for managed references, typically as 32-bit handles into a bounded managed heap window.
The MIR type remains `ref<T, managed, ...>` either way.
Both `ref<T, managed>` and `ref<T, managed, readonly>` are first-class and Lower preserves mutability information from DIR.
Owned handles follow the same rule: both `ref<T, owned>` and `ref<T, owned, readonly>` are representable.
Raw, borrowed, and owned references stay pointer-width because they participate in unsafe operations and FFI ABIs.

#### Raw Allocation

`raw.alloc` creates manually-managed heap memory for owned values (`^T`).
Cleanup uses `raw.drop` (ownership end + deallocate) or `raw.free` (manual deallocate).

```mir
type SomeType = struct { int64 }

v0 = raw.alloc SomeType -> ref<SomeType, raw, readonly>
; ... use v0 ...
raw.drop v0    ; ownership end + deallocate
```

For manual deallocation without ownership drop semantics (FFI, low-level code):

```mir
v0 = raw.alloc SomeType -> ref<SomeType, raw, readonly>
; ... use v0 ...
raw.free v0    ; just deallocate
```

No GC overhead.
Used with ownership annotations (`^T`) for Rust-like semantics.
In debug builds, a tracing allocator (like Zig's) can detect leaks, double-frees, and use-after-free.

#### Stack Allocation

`stack.alloc` creates frame-local storage.
Cleanup uses `stack.drop` for ownership end; deallocation happens automatically when the frame exits.

```mir
type SomeType = struct { int64 }

v0 = stack.alloc SomeType -> ref<SomeType, raw, readonly, addressSpace(stack)>
; ... use v0 ...
stack.drop v0    ; ownership end only, frame handles memory
```

No heap allocation.
The optimizer promotes `raw.alloc` to `stack.alloc` via escape analysis when the value doesn't escape the function.

### Drop Glue

The drop instructions (`raw.drop`, `stack.drop`) perform **drop glue**:

1. **Drop owned fields** in reverse declaration order (LIFO, like Rust/C++)
2. **Deallocate** (only for `raw.drop`; `stack.drop` skips this)

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
Lower has full DIR type information and generates the appropriate ownership cleanup behavior for each type.

**Important:** Lower only *marks* ownership on types and values (via `^T` modifiers and allocation instructions).
The actual drop instruction insertion happens in Optimize's `drop-insert` pass, which runs as part of the Verify phase.
This separation ensures drops are placed at precise last use points after all control flow is lowered.
`drop-insert` handles ownership lifetime cleanup only.
`using` protocol disposal is lowered independently of `raw.drop` and `stack.drop`.

| Instruction | Drop Fields | Call Dispose | Deallocate |
|-------------|-------------|--------------|------------|
| `raw.drop` | Yes (LIFO) | No | Yes |
| `stack.drop` | Yes (LIFO) | No | No (frame) |
| `raw.free` | No | No | Yes |

For managed allocations (`managed.alloc`), there is no drop instruction.
The GC handles cleanup, with finalizers for any `^T` fields (nondeterministic).

### Ownership

Destack aims to cover the "managedness" spectrum from TS to Go to Rust: implicit GC by default, explicit ownership when needed.
Most code just uses the default, and that should still be plenty fast thanks to real AOT compilation and fixed layouts (more like Go, Java, C#).
Performance critical code adds these ownership modifiers for manual control.
Source level ownership semantics are defined in [language/SPECIFICATION.md](../../../SPECIFICATION.md).

**Explicit Ownership:**

To preserve TypeScript semantics, a plain type `T` always follows the same rules as TypeScript (objects are GC managed references, primitives are values).
For reference types, that default `T` is a managed reference with logical object identity rather than guaranteed raw address semantics.

| Modifier | Semantics | After `foo(x)` | Who cleans up? |
|----------|-----------|----------------|----------------|
| `T` | GC managed (implicit) | `x` still valid | GC |
| `&readonly T` | Borrow (read only) | `x` still valid | Original owner |
| `&T` | Borrow (mutable) | `x` still valid, maybe changed | Original owner |
| `^T` | Owned reference (move-only) | `x` **invalid** | New owner |

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
    doWork(&readonly data)            // borrow it
    // data can be dropped before the next statement
}
```

Optimize's `drop-insert` pass inserts drops at last use points (non lexical), including before
control flow merges and before coroutine suspension when the value is not used after resume.

#### Borrowing

`&T` and `&readonly T` are explicit borrows of data.
They lower to borrowed reference types in MIR, which are address carrying values but not the same thing as raw pointers:

```ds
function process(data: &readonly Point) { ... }   // read only reference
function mutate(data: &Point) { ... } // mutable reference
```

Lowers to (conceptual):
```mir
type Point = struct { float32, float32 }

function process(v0: ref<Point, borrowed, readonly>): void { ... }
function mutate(v0: ref<Point, borrowed>): void { ... }
```

Lower treats locals, `this`, globals, and member or index access as addressable places for `&expr`.
Non addressable expressions are materialized into a temporary local before `local.address` is emitted.
If the borrow target is a member or index on a reference-like base, Lower uses the base value directly and avoids a spill.
Borrowing subfields lowers to explicit address projections (`field.address`, `element.address`).
For movable managed storage, borrowed addresses are only required to remain valid within the proven borrow lifetime.
Backends may rematerialize those addresses across safepoints or require pinning when code needs stable raw exposure.
Borrowed references are verified by the borrow check pass in Optimize.
Borrows are created by `field.address`, `element.address`, and by calls that return borrowed references with lifetimes.
A borrow ends when the reference value is no longer live.
Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
Dropping or freeing a value while it is borrowed is always an error.
In strict mode, conflicting borrows and invalidating stores are errors.
In lenient mode, the same situations produce warnings.

#### Raw pointers

`*T` and `*readonly T` are unsafe pointers with no borrow tracking.
They lower directly to `ref<T, raw>` and `ref<T, raw, readonly>`.
Deref and mutation use explicit `load`/`store` and pointer operations.
Conversions between borrowed references and raw pointers are explicit.
Address sensitive APIs, FFI boundaries, and layout critical code should use raw pointers directly or request pinned managed storage.

#### Address spaces

Lower preserves address space annotations on references for native and accelerator targets.
The default address space is `generic`.
Non generic address spaces are only valid for borrowed and raw references.
`constant` references are always immutable.
Address space changes are explicit and use the `addressSpace.cast` intrinsic.

#### Borrow Modes

By default, `&T` and `&readonly T` are hints and violations produce warnings.
They help document APIs, guide drops, and enable limited optimizations.
Set `borrowMode: "strict"` in `destack.json` to enforce exclusive `&` borrows.
Strict mode enables stronger `noalias` optimizations and errors on violations.
Strict mode forbids:
- Aliasing `&` with any other borrow
- Storing `&` inside managed objects
- Holding `&` across `await` or generator suspension

### Closures

Functions that capture variables become closure values:

```ds
const x = 10
const f = (y: int) => x + y  // captures x
```

Lowers to a function environment plus a function pointer to the original function.
The captured environment stores:
- `x: int64`

The MIR function value pairs the function pointer with the environment:
- `fnPtr: FunctionPointer`
- `env: ManagedReference<FunctionEnvironment>`

In MIR text this uses `closure(...) -> ...` for the callable value type.

**Capture semantics:**
- `const` bindings are captured by value (copied into closure struct)
- `let` bindings are captured by reference (pointer to original location)
- `@capture("byMove")` captures bindings by move into the environment
- This matches JavaScript's closure semantics

**Environment layout:**
Captured variables are stored in the closure struct in declaration order (order of first capture).
The struct is alignment-packed to minimize size.
Interior pointers are used for reference captures.

The closure body reads its environment via `function.environment`.
Closure calls use a `closure(...) -> ...` callable value with `call.indirect`.
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
JavaScript's `throw`/`catch` and `async`/`await` are powerful but have runtime costs: exception tables, stack unwinding, and state machine overhead.
**Result first error handling**: recoverable errors use `Result<T, E>` with zero-cost early returns in the common case, while `throw` remains available for compatibility and bug paths.
Explicit errors stay in the type system, and panics remain for bugs only.
Async functions lower to state machines, preserving JS `Promise` semantics without the JS runtime overhead.

### Errors and Exceptions

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while `throw` remains the compatibility and panic surface for exceptional control flow.

| Mechanism | Use For | Example |
|-----------|---------|---------|
| `Result<T, E>` | Recoverable errors | Parse failures, file not found, network timeout |
| `throw` | Panics, compatibility exceptions, and interop boundaries | Assertion failures, invariant violations, exception-based foreign APIs |

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

**Panic and throw:**
`throw` is represented explicitly in MIR as exceptional control flow.
Destack remains `Result` first, and panic paths are still expected to be rare.
The architectural target for native code is real exceptional control flow with explicit normal and unwind successors in MIR, even when a particular backend or runtime path still rejects or simplifies parts of that model during bring-up.
`trap.abort` and `trap.panic` remain the fatal control-flow forms for unrecoverable runtime termination.

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
| **Async library** | Store state, manage continuations, expose async APIs | `builtin/language/native/` |
| **Runtime** | Schedule async work and I/O primitives | `builtin/library/destack/` |

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
