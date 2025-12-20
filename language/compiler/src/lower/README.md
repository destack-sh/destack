# Lowering: DIR to MIR

This document describes how Destack's high-level semantic representation (elaborated, canonical DIR) is lowered to machine-level IR (MIR) for native and WASM targets (currently via Cranelift).

---

# Overview

## Objectives

The overarching goal is **Rust performance with TypeScript semantics and ergonomics**.
These are targets, not guarantees; actual performance depends on workload and optimization maturity:

- **Best case (target):** Rust-tier performance (zero-cost abstractions, no GC pauses)
- **Average case (target):** Go-tier performance (efficient GC, good concurrency)
- **Worst case (target):** Competitive with optimized JS runtimes (V8, JSC, SpiderMonkey)

AOT compilation provides predictable performance without warmup, but V8's speculative optimization can beat static compilation on some dynamic patterns. Our advantage is consistency and control.

Specifically, Destack lowering enables:
1. **TypeScript semantics**: TS and Destack code behaves identically in native
2. **Comptime**: Full compile-time evaluation
3. **Reflection**: Types-as-values for comptime and runtime reflection
4. **Ownership**: Manual memory or GC as needed
5. **Erasure**: Clean codegen to JS/TS

## Pipeline

The mid-end/back-end flow from DIR to MIR (see [compiler/README.md](../README.md)):
```
DIR (elaborated, canonical, target-independent)
 │
 ├─→ Generate/JS: direct JS/TS output (preserves type-erased polymorphism)
 │
 └─→ Lower (THIS DOCUMENT)
      │
      MIR (monomorphized, typed, target-specific)
       │
       ├─→ Verify: validate control flow, types, safety
       ├─→ Execute: run comptime blocks, substitute results
       ├─→ Optimize: inline, eliminate dead code, etc.
       │
       └─→ Generate
            ├─→ Cranelift → native binary (.exe, .dylib)
            └─→ WASM → WebAssembly module (.wasm)
```

The basic tasks of the lowering pass are:
1. **Monomorphize generics**: each `T` instantiation becomes specialised MIR
2. **Compute layouts**: property offsets, struct sizes, alignment
3. **Resolve dispatch**: builtin ops, direct calls, vtable lookups
4. **Generate type descriptors**: for reflection and instanceof
5. **Lower control flow**: expressions → blocks with terminators
6. **Allocate locals**: stack slots for variables and temporaries

### Input: Canonical DIR

Lower receives "canonical" typed DIR after Analyze and Elaborate (see [analyze/](../analyze/) and [elaborate/](../elaborate/)):
- All desugaring complete (e.g., `+=` → `+` and assign)
- All patterns expanded to decision trees
- All types fully inferred ("Types")
- All overloads resolved ("Resolutions")
- All static parameters resolved to values ("StaticExpression")
- All polymorphic instances created ("Instances")

### Output: Target-Specific MIR

MIR is generated per-target with target-specific decisions:
- Memory layout (LP64, ILP32, etc.)
- Calling conventions (C, System, Destack)
- Alignment requirements
- Endianness

---

# Interoperability

Destack aims for full **modern TypeScript** compatibility, and some dynamic JavaScript features are incompatible with ahead-of-time compilation. Dynamic features that interfere with AOT compilation are restricted or forbidden in native targets (but fully supported in JS codegen targets). Fortunately, most modern TS code already avoids such highly dynamic patterns as a best practice.

## Restrictions

JavaScript, as originally designed, is a highly dynamic language with dynamic scopes and litte typing guarantees. Over time, like in many dynamic languages, much of the community has come around to a restricted, statically typed variant of the language in modern TypeScript.

Critically, Destack aims to enable native compilation of **modern TypeScript**, not of arbitrary JavaScript.
There are other projects that attempt AOT JavaScript (with varying degrees of success), and such untyped dynamic code is explicitly out of scope and cannot be compiled.

### Exception Handling

**On native targets, `throw` aborts the process.** There is no stack unwinding, no catching.
This is caught by the compiler, and misusing `throw` is a compile error.
The divergence on exception handling is the most significant semantic difference between Destack and traditional JavaScript/TypeScript:

```
// this works in JS/TS:
try {
    throw new Error("oops")
} catch (e) {
    console.log("caught")  // executes
}

// On native Destack: process aborts at throw, catch never runs
```

**Why:** Zero-cost error handling. Exception tables and stack unwinding add overhead to every function call. By making `throw` an abort, the happy path has no exception-handling cost.

**What to use instead:** `Result<T, E>` with the `?` operator for recoverable errors:

```
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?    // propagates error
    Result.ok(parseConfig(text))
}
```

**JS target:** `throw` works normally for compatibility. You can still take advantage of Destack's many other features while keeping exceptions around at no extra cost.

**Interop:** Exceptions don't cross FFI boundaries. If calling JS that throws (via WASM), wrap it on the JS side to return a Result-like object. See [FFI Error Handling](#ffi-error-handling).

See [Errors and Exceptions](#errors-and-exceptions) for full details.

### Forbidden Features

Fully dynamic features are forbidden in native targets.

| Feature | Reason | Alternative |
|---------|--------|-------------|
| `eval()` | Arbitrary code execution | Comptime evaluation |
| `with` statement | Dynamic scope modification | Explicit object destructuring |
| `Proxy` | Intercepts all property access | Explicit wrapper types |
| `Reflect` | Runtime metaprogramming | Comptime reflection, RTTI |
| `__proto__` | Prototype chain mutation | Fixed type hierarchy |
| `Object.setPrototypeOf()` | Prototype chain mutation | Fixed type hierarchy |

Some of these features are already discouraged in modern TypeScript (strict mode forbids `with`; `eval` breaks type safety), others have solid alternatives as espoused by our standard library.

### Object.prototype Methods

Object prototype methods that depend on a dynamic prototype chain are generally not available:

| Method | Status | Alternative |
|--------|--------|-------------|
| `toString()` | Supported | Via `Display` interface |
| `valueOf()` | Not needed | Implicit coercion discouraged; use explicit conversion |
| `hasOwnProperty(key)` | Limited | Static keys only; use `in` operator or RTTI |
| `constructor` | Aliased | `obj.constructor` becomes `typeOf(obj)` |
| `isPrototypeOf()` | Forbidden | Use `instanceof` with RTTI |
| `propertyIsEnumerable()` | Forbidden | Use RTTI reflection |

For `toString()`, types implement the `Display` interface from `@destack-sh/core`:
```
newtype interface Display {
    display(): string
}
```

Primitives and common types have default `Display` implementations.
User types can implement `Display` explicitly or auto-derive it.

### Dynamic Property Access

Static property access (compile-time known keys) works normally:
```
point.x           // compile-time field offset
user.name         // compile-time field offset
```

Dynamic property access (runtime-computed keys) requires special handling:
```
obj[computedKey]  // not allowed on structs/classes
map[computedKey]  // works: Map implements Index<K, V>
record[key]       // works: Record<K, V> aliases to Map<K, V>
```

## Semantic Differences

Some TypeScript patterns have different semantics in native vs JS targets.

### Record Types

In TypeScript, `Record<string, T>` is an object with dynamic string keys:
```
const cache: Record<string, User> = {}
cache[id] = user   // dynamic property access
cache.get          // undefined (it's not a Map)
```

For native targets, objects have fixed layouts, so Destack aliases `Record<K, V>` to `Map<K, V>`:

```
// Source code (works on both targets)
const cache: Record<string, User> = {}
cache[id] = user   // desugars to cache.indexSet(id, user)
const u = cache[id] // desugars to cache.index(id)

// JS target: plain object
// Native target: Map<string, User>
```

This works transparently because `Map<K, V>` implements `Index<K, V>` and `IndexSet<K, V>`.

**What works:**
- `record[key]` and `record[key] = value` (via Index/IndexSet)
- `record.keys()`, `record.values()`, `record.entries()` (Map methods)
- `for (const [k, v] of record)` (Map is iterable)
- `JSON.stringify(record)` (serializes as JSON object, not Map)
- `key in record` (desugars to `record.has(key)`)

**What doesn't work (compile error on native):**
- `for (const k in record)` (use `for (const k of record.keys())`)
- `{...record}` spread (use `Map.from(record)` or explicit copying)
- `Object.keys(record)` (use `record.keys()`)
- `Object.assign(record, other)` (use `record.merge(other)` or loop)

**Trade-offs:**
- Hash map instead of JS object shape (different performance characteristics)
- Slightly different API surface

**Recommendation:** For data with known keys, prefer explicit struct types. Structs have fixed layout, predictable serialization, and identical behavior on all targets. Use `Record<K, V>` only when you genuinely need dynamic keys.

### JSON

JSON just works. Both `JSON.parse()` and `JSON.stringify()` are fully supported.

**JSON.parse()** returns `JsonValue`, a closed union of all valid JSON types:
```
type JsonValue = null | boolean | number | string | JsonValue[] | Record<string, JsonValue>
```

This is type-safe without requiring unbounded `any` because JSON has a known, finite set of value types.
Pattern matching on `JsonValue` gives you the concrete type:
```
const data = JSON.parse(text)  // JsonValue
match (data) {
    null => ...
    boolean b => ...
    number n => ...
    string s => ...
    JsonValue[] arr => ...
    Record<string, JsonValue> obj => obj["name"]
}
```

For typed parsing with validation, use the integrated schema library:
```
const user = User.parse(text)  // Result<User, ParseError>
```

**JSON.stringify()** generates serialization code at compile time based on the static type:
```
const user: User = ...
JSON.stringify(user)  // comptime generates User serialization
```

No runtime reflection needed; the compiler knows the type and emits field-by-field serialization.

### Symbol Property Keys

Symbols work as property keys when they're statically known:
```
const iter = obj[Symbol.iterator]  // compile-time known, works
obj[Symbol.toStringTag] = "MyType" // compile-time known, works
```

For dynamic symbol-keyed collections, use `Map<symbol, T>`:
```
const sym = Symbol("dynamic")
const map = Map<symbol, string>.new()
map[sym] = "value"  // explicit Map, not object property
```

### Array Methods

Array prototype methods work as expected. They're monomorphized per element type:

```
const nums: int[] = [1, 2, 3]
nums.map(x => x * 2)      // monomorphized: Array_int_map
nums.filter(x => x > 1)   // monomorphized: Array_int_filter
nums.reduce((a, b) => a + b, 0)
```

Methods that take callbacks receive closures. The closure type is also monomorphized:
- `map<U>((T) => U)` generates code for the specific `T` and `U`
- `sort((T, T) => int)` generates a comparator call for the specific `T`

Mutating methods (`push`, `pop`, `splice`, `sort`) work in-place on the array's backing storage.

### this Binding

JavaScript's `this` binding rules are preserved:

**Arrow functions** capture `this` lexically (from enclosing scope):
```
class Counter {
    count = 0
    increment = () => { this.count++ }  // this is always Counter instance
}
```
Lowers to a closure that captures `self` in its environment.

**Regular functions/methods** receive `this` as implicit first parameter:
```
class Counter {
    count = 0
    increment() { this.count++ }  // this passed at call site
}
```
Lowers to `@Counter_increment(self)`.

**Standalone functions** have `this = undefined` (strict mode):
```
function standalone() { return this }  // undefined
```

`.call()`, `.apply()`, `.bind()` work by manipulating the implicit `this` parameter:
```
fn.call(obj, arg)   // lowers to: fn(obj, arg)
fn.bind(obj)        // lowers to: closure capturing obj as this
```

---

# Types

Destack's type system maps TypeScript's structural, polymorphic types to native representations with concrete, monomorphic layouts.
We resolve this via **monomorphization**: each generic instantiation becomes specialized code with known sizes and offsets.
Zero-cost generics (no boxing, no vtables) at the cost of code size, the same tradeoff Rust and C++ make.
TypeScript semantics are preserved: structural compatibility still works, but at the MIR level everything has a concrete type.

## Monomorphization

DIR has Instance-level information from Analysis, but is still polymorphic. 
For native targets, we fully monomorphize generics like Rust.
Each generic instantiation gets its own specialized MIR logic.

```
function identity<T>(x: T): T { x }

identity<int32>(5)      // generates: identity_int32
identity<string>("hi")  // generates: identity_string
```

**Tradeoffs:** Monomorphization trades code size for runtime performance.

| Aspect | Benefit | Cost |
|--------|---------|------|
| No boxing | Zero overhead generics | More code copies |
| Known offsets | Direct field access | Larger binary |
| Inlining | Cross-generic optimization | Longer compile times |
| Register allocation | Optimal per-instantiation | More codegen work |

**Limitation:** Heavily generic code can cause code bloat. A function instantiated with
100 different types generates 100 copies. Dead code elimination removes unused
instantiations, but widely-used generics (e.g., `Option<T>`) will have many copies.

**Mitigation:** For very generic code where performance isn't critical, use `any` or
`unknown` to share implementations at the cost of runtime type checks.

The DIR `Instance` type tracks generic instantiations with their static arguments.
Lower receives an `Instance` and generates specialized MIR for that instantiation.

(For JS/TS codegen, we preserve polymorphic code with no monomorphization needed, except for
non-erasable value parameters like `const N: int`.)

## Resolution

Resolution tells Lower *which symbol* is being called at a call site.
This is determined by Analyze and attached to DIR nodes.
- **Resolution** answers: "Which declaration are we targeting?"
- **Dispatch** answers: "How do we invoke that target at runtime?"

Even with `Resolution::Static`, the target may require vtable dispatch if it's a virtual method on a polymorphic type.
`Resolution::Dynamic` is specifically for *union types* where different union variants call different symbols.

### Resolution::Builtin

Primitive operations on builtin types.
No function call needed; Lower emits MIR instructions directly.

```
a + b  // Resolution::Builtin for int32 + int32
```

Lowers to:
```mir
v2 = iadd v0, v1
```

### Resolution::Static

Single known target symbol.
The *symbol* is known, but dispatch may still be virtual or direct depending on the method.

**Direct dispatch** (function, non-virtual method, final class, struct method):
```
user.getName()  // Resolution::Static, non-virtual
```

Lowers to:
```mir
v1 = call @User_getName(v0)
```

**Virtual dispatch** (virtual method on class/interface):
```
animal.speak()  // Resolution::Static { target: Animal::speak }, but virtual
```

Lowers to vtable lookup:
```mir
v1 = field.get v0, 0           ; load vtable pointer from object header
v2 = field.get v1, 2           ; load speak method at vtable slot 2
v3 = call.indirect v2(v0)      ; indirect call through vtable
```

The key: Resolution::Static means we know *which method signature* (Animal::speak), but if it's virtual, the actual implementation depends on the concrete type.

### Resolution::Dynamic

Union-based dispatch: different union variants call different symbols.
Elaborate has already generated the dispatch logic; Lower just emits it.

**Union method dispatch:**
```
function process(x: Cat | Dog) { x.speak() }
// Cat::speak and Dog::speak are different symbols
```

Elaborate transforms to instanceof chain, Lower emits:
```mir
function @process_speak(v0: ref<Cat | Dog>) -> ref<string> {
block0(v0: ref<Cat | Dog>):
    v1 = field.get v0, 0       ; load type_id from header
    switch v1, block3, 0 => block1, 1 => block2
block1:
    v2 = call @Cat_speak(v0)
    jump block4(v2)
block2:
    v3 = call @Dog_speak(v0)
    jump block4(v3)
block3:
    unreachable              ; exhaustive match
block4(v5: ref<string>):
    return v5
}
```

## Type Representation

### Primitives

| DIR Type | MIR Type | Native Representation |
|----------|----------|----------------------|
| `boolean` | `Boolean` | `i8` (0 or 1) |
| `int8..int64` | `Int { width, signed: true }` | Native integer |
| `uint8..uint64` | `Int { width, signed: false }` | Native integer |
| `float32`, `float64` | `Float { width }` | IEEE 754 float |
| `void` | `Void` | Zero-sized |

### Overflow Behavior

Integer arithmetic has defined overflow semantics (unlike C, inspired by Zig and Rust).

**Default (debug mode):** Trap on overflow.
Like Zig and Rust debug builds, arithmetic operations trap on overflow.
This catches bugs early with clear error messages.

**Release mode:** Configurable per-project.
Options: trap (safest), wrap (fastest), or check-and-handle.

**Explicit operators:** Always available regardless of mode.
- `+%`, `-%`, `*%`: wrapping (two's complement wrap)
- `+|`, `-|`, `*|`: saturating (clamp to min/max)

```
const a: uint8 = 250;
const b: uint8 = 10;

a + b       // traps in debug, behavior depends on config in release
a +% b      // always wrapping: 250 + 10 = 4
a +| b      // always saturating: 250 + 10 = 255
```

MIR representation:
- Default ops use `Binary` with overflow checking intrinsic in debug
- Wrapping ops use `Intrinsic::WrappingAdd` etc.
- Saturating ops use `Intrinsic::SatAdd` etc.

### Null and Undefined

TypeScript has both `null` and `undefined`. For native:

**Representation:**
- `null`: Zero/null pointer (0x0)
- `undefined`: Distinguished sentinel value (0x1 or special pattern)

**Nullable types** (`T | null` or `T?`):

For pointer types, we use **niche optimization** (like Rust's `Option<&T>`).
The null pointer (0x0) is an invalid address for valid objects, so we can use it
as the "none" discriminant without adding a tag byte:

```
T?  where T is reference type  →  same size as T, null = 0x0
```

This is the same optimization Rust uses for `Option<Box<T>>`, `Option<&T>`, etc.
The "niche" is the invalid bit pattern (null pointer) that we repurpose as a discriminant.

For value types, there's no invalid bit pattern to exploit, so we need a tag:
```
int?  →  { tag: u8, value: int }  // 2 bytes overhead minimum
```

**Niche optimizations:**

Like Rust, we can exploit invalid bit patterns to save space in type layouts.
Beyond null pointers, we can exploit other invalid bit patterns:
- `boolean | null` → use value 2 for null (bool only uses 0 and 1)
- `character | null` → use invalid Unicode scalar values
- Enums with < 256 variants → use unused discriminant values

**Discriminated union optimization:**

When `null` is one variant of a union, we can often use the null pointer:
```
Result<T, null>  →  T? (just use null for error case, no tag needed)
```

### BigInt

TypeScript's `bigint` is arbitrary-precision.
For native, we use a tagged pointer representation with small-integer optimization in cases where object identity is not required:

```
┌─────────────────────────────────────────────────────┐
│ Small bigint (fits in 63 bits):                     │
│   [63-bit value][1-bit tag=1]                       │
│   No heap allocation, inline arithmetic             │
├─────────────────────────────────────────────────────┤
│ Large bigint (> 63 bits):                           │
│   [pointer to limb array][1-bit tag=0]              │
│   Heap-allocated, arbitrary precision               │
└─────────────────────────────────────────────────────┘
```

```
const small: bigint = 42n          // inline: 0x0000000000000055 (42 << 1 | 1)
const large: bigint = 2n ** 100n   // heap: pointer to limb array
```

The native `bigint` has TypeScript semantics:
- `===` compares values, not identity (bigints are value types semantically, same as JS/TS)
- Small bigints compare inline; large bigints compare limb-by-limb
- Overflow from small to large is automatic and transparent (same layout size)

### Symbol

TypeScript's `symbol` is a unique identifier.
For native, symbols are interned integers:

```
const symbol = Symbol("description")
```

MIR representation:
- `symbolId: uint64` (unique per-symbol, assigned at creation)
- Description string stored separately in symbol table

Symbol comparison is just integer comparison.
`Symbol.for()` looks up in a global string-to-symbol map.

### Strings

Strings are immutable, GC-managed byte sequences.
For native targets, we use UTF-8 encoding internally.

#### String Layout

```
/// Native string representation
struct String {
    header: GCHeader          // GC metadata (mark bits, etc.)
    lengthUtf16: uint32       // UTF-16 code unit count (for TS compatibility)
    lengthBytes: uint32       // byte length of UTF-8 data
    hash: uint32              // cached hash (computed lazily)
    data: uint8[]             // UTF-8 encoded bytes (inline, variable length)
}
```

**Size:** header (16 bytes typically) + N bytes data (variable)

For TypeScript semantic compatibility, we emulate the TypeScript UTF-16 methods:
- `.length` returns UTF-16 code unit count (not bytes, not codepoints)
- `.charAt(i)` indexes by UTF-16 code units (requires UTF-8 → UTF-16 translation)
- `.charCodeAt(i)` returns UTF-16 code unit

We store UTF-8 for memory efficiency (ASCII text is half the size of UTF-16), but
TypeScript's indexing semantics use UTF-16 code units. 
For non-ASCII strings, this
means `charAt(i)` requires O(n) scanning or cached index tables.

ASCII-only strings (detected at creation) use O(1) indexing because byte index == UTF-16 index. 
This covers the vast majority of real-world strings.

#### String Literals

String literals are interned at compile time and stored in a read-only data section:

```
const s = "hello"   // pointer to static data (never collected)
```

Literal strings have their hash precomputed and share storage across the program.

#### Template Literals

```
`Hello, ${name}!`
```

Lowers to a series of string concatenations or a builder pattern:
```mir
function @template_example(v0: ref<String>) -> ref<String> {
block0(v0: ref<String>):
    v1 = global.const @str_Hello    ; "Hello, "
    v2 = call @String_concat(v1, v0) ; "Hello, " + name
    v3 = global.const @str_Bang      ; "!"
    v4 = call @String_concat(v2, v3) ; result + "!"
    return v4
}
```

### Arrays

Arrays in Destack are monomorphic after lowering (i.e., we know and enforce specific element types, even if that element type is boxed or `unknown`).

**Fixed-size arrays** `T[N]`:
```
┌─────────────────────────────────────────┐
│ Inline, no header, known size at compile│
│   elements: [T; N]                      │
│   Size = N * sizeof(T)                  │
└─────────────────────────────────────────┘
```

**Dynamic arrays** `T[]`:
```
┌─────────────────────────────────────────┐
│ Heap-allocated, growable                │
│   header: ...           (GC metadata)   │
│   length: uint32                        │
│   capacity: uint32                      │
│   data: T[capacity]                     │
└─────────────────────────────────────────┘
```

| Pattern | MIR Type | Notes |
|---------|----------|-------|
| `T[N]` | `Type::Array { element, length: N }` | Inline, value semantics |
| `T[]` | `Type::ManagedReference` to array | Heap, reference semantics |
| `TypedArray` | `Type::RawPointer` to buffer | Direct memory access |

**Important:** `T[]` is NOT `Array<unknown>`. After monomorphization, we know T.
`Array<unknown>` or `any[]` boxes elements and uses RTTI for type checks.

**Array operations:**
```mir
; a[i] where a: int[], a is v0, i is v1
v2 = field.get v0, 2        ; get data pointer
v3 = element.get v2, v1     ; load element at index

; a.push(x) where x is v4
call @Array_push(v0, v4)

; a.length
v5 = field.get v0, 1        ; load length field
```

### Structs and Classes

Both structs and classes are **reference types by default** in Destack (like TypeScript objects).
Ownership modifiers (`^T`, `&T`) are orthogonal and can force value or reference semantics on either.

| Aspect | struct | class |
|--------|--------|-------|
| Reference identity | No (`===` is compile error) | Yes (`===` compares pointers) |
| Type identity | Optional (`typeId` only when needed) | Always (has `typeId`) |
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

The `final` keyword on methods or classes is an API contract ("you may not override/extend"), not an optimization hint. 
For whole-program compilation, the optimizer already knows what's overridden. `final` matters for libraries where downstream users could extend your classes.

Both lower to `Type::Struct` with computed property offsets. The key difference is **reference identity**: classes have it (two instances with same data are still different objects), structs don't (two structs with same data are equal). Both can have **type identity** (RTTI) when needed for `instanceof` or `typeOf`.

#### Struct Layout

Structs have no **reference identity** (no `===`), but they can have **type identity** (RTTI) when needed.
RTTI is only emitted for structs that are used with `instanceof`, `typeOf`, or stored in polymorphic contexts.

```
struct Point { x: float32, y: float32 }

// native layout WITHOUT RTTI (when instanceof not needed)
struct PointLayout {
    header: GCHeader,        // GC metadata (8 bytes typical)
    x: float32,              // offset 8
    y: float32,              // offset 12
}
// total size: 16 bytes (with padding)

// native layout WITH RTTI (when instanceof is used)
struct PointLayoutWithRTTI {
    header: GCHeader,        // GC metadata (8 bytes typical)
    typeId: uint32,          // for instanceof (offset 8)
    x: float32,              // offset 12
    y: float32,              // offset 16
}
// total size: 20 bytes (with padding)
```

Structs are data-oriented types optimized for value semantics.
Two structs with the same properties are equal by value (`==`), and reference comparison (`===`) is a compile error.

**When RTTI is emitted for structs:**
- Used with `instanceof` on a value of unknown/union type
- Used with `typeOf()` at runtime
- Stored in `any` or `unknown`
- Part of a union type that requires runtime discrimination

**When to use discriminated unions instead:**
For performance-critical code where you want to avoid RTTI overhead, use discriminated unions with an explicit tag:

```
struct Cat { kind: "cat" = "cat", name: string }
struct Dog { kind: "dog" = "dog", name: string }
type Pet = Cat | Dog

function greet(pet: Pet) {
    match (pet.kind) {
        "cat" => print("meow")
        "dog" => print("woof")
    }
}
```

The tag approach uses string interning (see [String Tag Interning](#string-tag-interning)) for efficient dispatch.

#### Class Layout

Classes always have **both** reference identity (`===`) and type identity (RTTI).
This enables runtime type checks, polymorphic method calls, and identity comparison:

```
class Animal {
    name: string;
    speak(): string { "..." }
}

// native layout (heap-allocated)
struct AnimalLayout {
    header: GCHeader,        // GC metadata (8 bytes)
    typeId: uint32,          // for instanceof (offset 8)
    vtablePtr: &VTable,      // for virtual dispatch (offset 12 or 16)
    name: ref<String>,       // first user field
}
```

#### GC Header

The GC header is present on all managed heap objects:

```
struct GCHeader {
    markBits: uint8,         // tri-color mark state
    flags: uint8,            // pinned, finalizer, etc.
    padding: uint16,         // alignment
    sizeClass: uint32,       // allocation size class (for fast free)
}
// size: 8 bytes
```

**Explicit value semantics:**
Use `^T` to force value/copy semantics:
```
function process(point: ^Point) {    // ^Point = value type, point is copied
    // modifications don't affect caller
}
```

Field layout is computed during lowering based on target alignment requirements.

### Class Inheritance

Classes with `extends` get special handling for field layout and method dispatch.

**Field layout:** Parent fields come first, then child fields.

```
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}
```

Dog's MIR layout:
- offset 0: ObjectHeader
- offset 8: name (from Animal)
- offset 16: breed (from Dog)

This ensures a `Dog` pointer can be used where an `Animal` pointer is expected.

**Method dispatch:** Classes use vtables for virtual methods.

```
class Animal {
    speak(): string { "..." }
}

class Dog extends Animal {
    speak(): string { "woof" }
}
```

### VTable Layout

Each class with virtual methods has a static vtable.
The vtable is an array of function pointers, one per virtual method.

**VTable structure (conceptual):**
```
struct VTable {
    type_id: uint32              // for instanceof
    destructor: () => void       // cleanup function
    methods: ((...args) => any)[] // virtual method pointers
}
```

**Example vtable layout:**
```
Animal vtable:
  slot 0: type_id = ANIMAL_TYPE_ID
  slot 1: destructor = Animal_drop
  slot 2: speak = Animal_speak

Dog vtable (inherits Animal):
  slot 0: type_id = DOG_TYPE_ID
  slot 1: destructor = Dog_drop
  slot 2: speak = Dog_speak      // overrides Animal::speak
```

**Slot assignment:**
- Slots are assigned in declaration order, starting from parent
- Child classes inherit parent's slot assignments
- Overriding methods use the same slot as parent
- New methods get new slots after inherited ones

**Virtual call lowering:**
```mir
; animal.speak() where animal could be Animal or Dog
v1 = field.get v0, 0           ; load vtable pointer from object
v2 = field.get v1, 2           ; load speak method (slot 2)
v3 = call.indirect v2(v0)      ; call with self as first arg
```

**Interface vtables:**
Interfaces also use vtables, but objects carry a fat pointer:
```
{ object_ptr: *Object, vtable_ptr: *InterfaceVTable }
```

Each (Type, Interface) pair has its own interface vtable mapping interface methods to concrete implementations.

**Super calls:** Compile to direct calls to parent implementation.

```
class Dog extends Animal {
    speak(): string { super.speak() + " woof" }
}
```

Lowers to:
```mir
function @Dog_speak(v0: ref<Dog>) -> ref<string> {
block0(v0: ref<Dog>):
    v1 = call @Animal_speak(v0)   ; direct call, no vtable lookup
    v2 = iconst " woof"
    v3 = call @String_concat(v1, v2)
    return v3
}
```

No vtable lookup needed for `super`; the target is statically known.

### Getters and Setters

Property accessors lower to method calls.
There is nothing special about them at the MIR level.

```
class Circle {
    #radius: float64

    get area(): float64 { 3.14159 * this.#radius * this.#radius }
    set radius(r: float64) { this.#radius = r }
}

c.area          // call @Circle_get_area(c)
c.radius = 5    // call @Circle_set_radius(c, 5)
```

### Tuples

Tuples lower to anonymous `Type::Struct` with indexed fields.

```
(int, string, bool)  →  Struct { fields: [i64, String, i8] }
```

## Union Types

Destack supports TypeScript's structural unions in all forms.
Naturally, the relative optimality of each union kind depends on the union shape and the target platform.

### Discriminated Unions (Tagged)

When all variants share a discriminant field (e.g., `kind`), use inline tagging:

```
type Result<T, E> = { kind: 'ok', value: T } | { kind: 'err', error: E }
```

Lowers to a MIR struct with:
- `tag: uint8` (0 = ok, 1 = err)
- `payload: uint8[N]` (max of sizeof(T), sizeof(E))

The `tag` field is the discriminant.
Pattern matching becomes a switch on tag.

#### String Tag Interning

TypeScript-style discriminated unions use string literals as tags:
```
{ kind: 'loading' } | { kind: 'success', data: T } | { kind: 'error', msg: string }
```

This is a very common pattern in TypeScript, and we can aggressively optimize it for native targets.
We intern these string tags to integer discriminants at compile time; conceptually this looks like this:

```
// compile-time tag mapping
const TAG_LOADING: uint8 = 0   // "loading"
const TAG_SUCCESS: uint8 = 1   // "success"
const TAG_ERROR: uint8 = 2     // "error"

// reverse mapping for runtime string access
const TAG_STRINGS: string[] = ["loading", "success", "error"]

// runtime representation
struct LoadingState { tag: uint8, /* no payload */ }
struct SuccessState<T> { tag: uint8, data: T }
struct ErrorState { tag: uint8, msg: string }
```

**Operations:**
- `x.kind === "success"` → `x.tag == TAG_SUCCESS` (fast integer compare)
- `console.log(x.kind)` → `TAG_STRINGS[x.tag]` (string lookup only when needed)

This preserves TS semantics while enabling efficient native dispatch.

#### TypeId Interning

We use the same interning mechanism for type identifiers (`TypeId`).
At the source level, `TypeId` is a string like `"myapp/models:User"`.
At runtime, it's an interned integer for fast comparison:

```
// source-level API
newtype TypeId = string;   // "myapp/models:User"

// compile-time interning
const TYPEID_USER: uint32 = 42;         // interned id for "myapp/models:User"
const TYPEID_ORDER: uint32 = 43;        // interned id for "myapp/models:Order"

// reverse mapping for reflection
const TYPEID_STRINGS: string[] = [..., "myapp/models:User", ...];
```

This unifies discriminated union tags and type identifiers under a single
string interning mechanism, reducing complexity and code duplication.

### Optimized Unions (Nullable, Primitives)

Nullable reference types use **niche optimization** (no tag needed in the type layout):

```
string | null       →  ManagedReference<String> (null = 0x0, no tag)
User | null         →  ManagedReference<User> (null = 0x0, no tag)
```

Value type unions require an explicit tag:

```
int32 | bool        →  { tag: u8, value: int64 }  // different value types
int32 | null        →  { tag: u8, value: int32 }  // value type + null
```

### General Unions (Boxed)

For complex unions without natural discriminant, values are boxed:

```
any                 →  { type_id: uint32, data: RawPointer<void> }
unknown             →  same as any, but type-checked
A | B | C           →  boxed if A, B, C are structurally incompatible
```

The `type_id` indexes into the RTTI table for type checking.

### Reflection

Destack's types-as-values feature makes `Type<T>` a first-class value, enabling
both compile-time and runtime reflection (as needed). 
This section describes how reflection is implemented at the MIR level.

**Source-level API** (from `@destack-sh/core/reflection`):
```
Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct"
    name: string
    id: TypeId              // stable identifier: "myapp/models:User"
    properties: Property[]
    decorators: DecoratorInfo[]
}
```

**Comptime:** Full type information is available as compile-time data.
Type operations execute in the compiler; no runtime representation needed.

```
const PROP_COUNT = comptime User.properties.length;    // → literal 3
const HAS_NAME = comptime User.properties.some(p => p.name == "name")  // → true

comptime if (User.properties.some(p => p.type == string)) {
    // branch selected at compile time, other branch eliminated
}
```

**Runtime:** When type information is needed at runtime, we generate RTTI.
The RTTI table is a static array embedded in the binary.
The native RTTI representation is a compact binary format that maps to the high-level
`Type<T>` API from `@destack-sh/core/reflection` (see `language/builtin/core/reflection/type.ds`):

```
// high-level API (source-level, what users see, see language/builtin/core/reflection/type.ds)
newtype Type<T> = StructType<T> | ClassType<T> | EnumType<T> | ...

struct StructType<T> {
    kind: "struct" = "struct"
    name: string
    id: TypeId                  // "myapp/models:User"
    properties: readonly Property[]
    decorators: readonly DecoratorInfo[]
    description?: string
}

// native RTTI (binary format, what the runtime uses)
struct TypeDescriptor {
    id: uint32                  // index into RTTI table
    typeIdOffset: uint32        // offset to TypeId string ("myapp/models:User")
    nameOffset: uint32          // offset to name string ("User")
    size: uint32                // sizeof(T) in bytes
    alignment: uint16           // alignof(T) in bytes
    kind: uint8                 // maps to Type<T> discriminant
    flags: uint8                // nominal, sealed, etc.
    parentId: uint32            // for class inheritance (0 if none)
    vtablePtr: &VTable          // for virtual dispatch (null if none)
    propertiesOffset: uint32    // offset to PropertyDescriptor array
    propertyCount: uint16       // number of properties
    decoratorsOffset: uint32    // offset to DecoratorDescriptor array
    decoratorCount: uint16      // number of decorators
}

struct PropertyDescriptor {
    nameOffset: uint32          // offset into string table
    typeId: uint32              // TypeDescriptor id for property type
    offset: uint32              // byte offset within parent struct
    flags: uint8                // optional, readonly, etc.
}
```

At runtime, when user code accesses `User.properties` or `typeOf(value)`, we synthesize
the full `Type<T>` structure from the compact `TypeDescriptor` on demand.

**Lowering Type<T> operations:**

| Source | Comptime | Runtime (if needed) |
|--------|----------|---------------------|
| `User` (in type position) | Type check | N/A |
| `User` (in value position) | Constant TypeDescriptor* | Load from RTTI table |
| `User.name` | Constant "User" | `rtti[user_id].name` |
| `User.properties` | Constant array | Load property descriptors |
| `value instanceof User` | Eliminated if type known | Compare `value.type_id == user_id` |
| `typeOf(value)` | Constant if type known | Load `value.type_id`, lookup in table |

**RTTI generation rules:**
RTTI is only emitted for types that need it at runtime:
- Types used with `instanceof` on unknown values
- Types used with `typeOf()` on unknown values
- Types stored in `any` or `unknown`
- Types with runtime reflection (non-comptime `.properties`, `.name`, etc.)

If all type operations are comptime, no RTTI overhead appears in the binary.
Dead code elimination removes unused RTTI entries.

**Relationship to `@destack-sh/core/reflection`:**
The source-level `Type<T>` API (StructType, ClassType, etc.) is richer than native RTTI.
For comptime operations, the full API is available.
For runtime, we generate minimal RTTI and synthesize the full Type<T> structure on demand.

---

# Dispatch

Method calls are resolved and dispatched based on the type system's structural or nominal nature.
TypeScript's duck typing means any object with matching methods can satisfy an interface, which creates challenges for native codegen.
**Structural interfaces** use fat pointers (TypeScript compatible duck typing) while **nominal interfaces** use thin pointers (explicit `implements`, simpler dispatch).
Destack supports both: structural by default for TS compatibility, nominal via `newtype interface` for performance.
Union dispatch generates type checking code when a value could be multiple types.

## Dynamic Dispatch

### Method Calls on Interfaces

Interfaces in Destack can be **structural** (default) or **nominal** (`newtype interface`).
Both use vtable-based dispatch, but with different representations.

#### Structural Interfaces (Fat Pointers)

Structural interfaces require **fat pointers** because the vtable layout varies per (Type, Interface) pair:

```
interface Drawable { draw(): void }
interface Resizable { resize(w: int, h: int): void }

struct Circle { radius: float }
struct Rectangle { width: float, height: float }
```

When a `Circle` is used as `Drawable`, we create a fat pointer:

```
// fat pointer representation
struct InterfaceRef<I> {
    objectPtr: &void          // pointer to the actual object
    vtablePtr: &InterfaceVTable<I>  // pointer to interface vtable
}
```

Each (Type, Interface) pair generates its own vtable:

```
// Circle as Drawable
const Circle_Drawable_vtable: InterfaceVTable<Drawable> = {
    typeId: CIRCLE_TYPE_ID,
    draw: @Circle_draw
}

// Rectangle as Drawable
const Rectangle_Drawable_vtable: InterfaceVTable<Drawable> = {
    typeId: RECTANGLE_TYPE_ID,
    draw: @Rectangle_draw
}
```

**Interface call lowering:**

```
function render(d: Drawable) { d.draw() }
```

Lowers to:
```mir
function @render(v0: ref<Drawable>) -> void {
block0(v0: ref<Drawable>):
    ; v0 is a fat pointer: { objectPtr, vtablePtr }
    v1 = field.get v0, 0       ; load objectPtr
    v2 = field.get v0, 1       ; load vtablePtr
    v3 = field.get v2, 1       ; load draw method from vtable slot 1
    call.indirect v3(v1)       ; call with object as self
    return
}
```

**Creating interface references:**

```
const c = Circle { radius: 5.0 }
const d: Drawable = c  // creates fat pointer
```

Lowers to:
```mir
function @example() -> ref<Drawable> {
block0:
    v0 = managed.alloc Circle
    v1 = iconst 5.0f32
    v2 = field.set v0, 0, v1         ; set radius
    ; create fat pointer (inline tuple, no heap allocation)
    v3 = global.const @Circle_Drawable_vtable
    v4 = stack.alloc InterfaceRef    ; allocate fat pointer on stack
    v5 = field.set v4, 0, v0         ; objectPtr
    v6 = field.set v5, 1, v3         ; vtablePtr
    return v6
}
```

#### Nominal Interfaces

Nominal interfaces (`newtype interface`) work like class vtables; the vtable pointer
is stored in the object header, not in a fat pointer. This is more efficient but
requires explicit `implements` declarations.

```
newtype interface Hashable {
    hash(): uint64
}

extension for Circle implements Hashable {
    hash(): uint64 { ... }
}
```

For nominal interfaces, objects carry their vtable, so we use thin pointers:

```mir
function @hash_it(v0: ref<Hashable>) -> i64 {
block0(v0: ref<Hashable>):
    ; v0 is a thin pointer; vtable is in object header
    v1 = field.get v0, 1       ; load vtablePtr from object header
    v2 = field.get v1, 2       ; load hash method
    v3 = call.indirect v2(v0)
    return v3
}
```

#### VTable Generation

For each (Type, Interface) pair where the type implements the interface:

1. Create a static vtable with method pointers in interface declaration order
2. Store the vtable as a global constant
3. When creating an interface reference, pair the object with the appropriate vtable

**Vtable layout:**
```
struct InterfaceVTable<I> {
    typeId: uint32            // for instanceof on interface refs
    methods: [FunctionPointer] // one per interface method, in declaration order
}
```

#### Cost Model

| Operation | Structural Interface | Nominal Interface | Class Virtual |
|-----------|---------------------|-------------------|---------------|
| Reference size | 2 pointers (fat) | 1 pointer (thin) | 1 pointer |
| Method call | 2 loads + indirect | 2 loads + indirect | 2 loads + indirect |
| Creation | Construct fat ptr | Just cast | Just cast |
| Memory per type | vtable per interface | vtable in object | vtable in object |

Structural interfaces enable TypeScript's duck typing but have overhead:
- 2× pointer size for interface references (fat pointer)
- One vtable per (Type, Interface) pair (can add up with many combinations)
- Cache locality may suffer from double indirection

### Union Dispatch

When a `Resolution` is `Dynamic` in DIR (multiple possible implementations),

1. Load `typeId` from object header
2. Compare against expected type IDs
3. Branch to appropriate implementation

For small unions, this is a switch on tag.
For RTTI-based unions, it's a type ID comparison.

---

# Memory

Memory allocation and ownership at the MIR level.
TypeScript/JavaScript uses garbage collection with no explicit memory management.
Destack preserves this simplicity by default (GC managed heap allocation), but enables opt in control for performance critical code.
**GC** is simple and TS compatible but has potential pauses; **manual/RC** is deterministic with no pauses but more complex.
Ownership modifiers (`&T`, `^T`) enable Rust-like control when needed, while escape analysis automatically optimizes the common case.

## Memory Management

Destack supports three allocation modes, controllable via `AllocationMode`:

| Mode | Allocations Allowed | Use Case |
|------|---------------------|----------|
| `Any` | All | Default, full TS compatibility |
| `NoManaged` | `RawAlloc`, `StackAlloc` | Realtime-safe, no GC pauses |
| `StackOnly` | `StackAlloc` | Embedded, deterministic |

### Managed Allocation (GC)

`ManagedAlloc` creates GC-tracked objects.
For WASM-GC compatible targets, we can just use WasmGC's `alloc` and `free` intrinsics directly without having to implement our own GC.
For native targets, the runtime uses a **concurrent tri-color mark-and-sweep** garbage collector, similar to Go's GC.

#### GC Algorithm

We use a concurrent, non-generational, tri-color mark-and-sweep collector (inspired by Go's GC):

| Property | Choice | Rationale |
|----------|--------|-----------|
| Concurrent | Yes | Sub-millisecond pauses, critical for interactive apps |
| Generational | No | Simpler, Go proves it works; can add later |
| Compacting | No | Avoids pinning complexity; fragmentation acceptable |
| Tri-color | Yes | Enables concurrent marking without stopping mutators |

**Why this design:**
- Go's GC achieves sub-millisecond pauses with this approach
- Our design enables aggressive escape analysis optimizations to avoid managed alloc in the first place
- Non-generational is simpler and avoids write barrier overhead for young→old pointers
- Concurrent marking allows application threads to continue during most of GC

#### GC Phases

The GC algorithm is conceptually simple and directly inspired by Go's GC.
```
1. Mark Setup (STW, ~100μs)
   - Enable write barrier
   - Scan stacks for roots
   - Grey all root objects

2. Concurrent Mark
   - Mutators run with write barrier
   - GC workers trace grey objects
   - Objects transition: white → grey → black

3. Mark Termination (STW, ~100μs)
   - Drain remaining grey objects
   - Disable write barrier

4. Concurrent Sweep
   - Reclaim white (unreachable) objects
   - Mutators can allocate during sweep
```

#### Tri-Color Invariant

Objects have three colors during marking:
- **White**: Not yet seen (potentially garbage)
- **Grey**: Seen but children not yet scanned
- **Black**: Scanned, all children are grey or black

The invariant: no black object points to a white object.
Write barriers maintain this invariant during concurrent marking.

#### Write Barriers

During concurrent marking, we need write barriers to maintain the tri-color invariant.
When a pointer field is written, we must ensure the pointed-to object is not white:

```mir
; x.field = y  (during GC mark phase)
function @write_barrier_example(v0: ref<Object>, v1: ref<Object>) -> void {
block0(v0: ref<Object>, v1: ref<Object>):
    ; if gc_phase == marking && is_white(v1):
    ;     shade_grey(v1)
    intrinsic.gc_write_barrier(v1)
    v2 = field.set v0, 1, v1
    return
}
```

The write barrier is an **insertion barrier** (Dijkstra-style): when writing a pointer,
we shade the *new* value grey. This ensures newly-reachable objects aren't missed
by the concurrent marker.

**Implementation:** Write barriers are automatically inserted by Lower for all
`ManagedReference` field writes. They're implemented via the `Intrinsic::GcWriteBarrier`
intrinsic, which the runtime handles. For advanced use cases (FFI, custom allocators),
the barrier intrinsics are available explicitly:

- `Intrinsic::GcWriteBarrier`: shade object grey if GC is marking
- `Intrinsic::GcReadBarrier`: (optional) for read barriers in some GC designs

**Optimization:** Write barriers are only active during the mark phase.
The barrier checks a global flag and fast-paths when GC is not marking.

#### Roots

GC roots are locations that keep objects alive:
- Stack slots containing managed references
- Global variables containing managed references
- Thread-local storage
- Finalizer queues

Stack scanning uses conservative pointer finding with type information from MIR.
Each function has a stack map describing which slots contain managed references.

#### Allocation

Objects are allocated from thread-local allocation buffers (TLABs) for fast, lock-free allocation:

```mir
function @alloc_example() -> ref<Point> {
block0:
    v0 = managed.alloc Point    ; fast path: bump TLAB pointer
    return v0                    ; slow path: request new TLAB or trigger GC
}
```

#### Explicit Reference Counting

For scenarios where GC pauses are unacceptable or deterministic destruction is needed,
use the standard library `Rc<T>` and `Arc<T>` types (like Rust):

```
const shared = Rc.new(data)       // explicit RC, single-threaded
const threadSafe = Arc.new(data)  // atomic RC for threading
const weak = Rc.downgrade(shared) // weak reference, breaks cycles
```

These are explicit opt-in types for specific situations, not automatic for all objects.
On JS targets, types like these are not necessary and just comptime-if down to regular references. 

### Raw Allocation

`RawAlloc` / `RawFree` for manual memory management:

```mir
v0 = raw.alloc SomeType
; ... use v0 ...
raw.free v0
```

No GC overhead.
Used with ownership annotations (`^T`) for Rust-like semantics.

NOTE: In debug builds, a tracing allocator (like Zig's) can detect leaks, double-frees, and use-after-free.

### Stack Allocation

`StackAlloc` for frame-local storage:

```mir
v0 = stack.alloc SomeType
; v0 is automatically freed when function returns
```

No heap allocation.
Used for temporary values, small structs.

## Ownership and Value Semantics

Destack covers the spectrum from TS to Go to Rust: implicit GC by default, explicit ownership when needed.
Most code uses the default. Performance-critical code adds hints. Systems code opts into explicit control.

### Ownership Options

To preserve TypeScript semantics, a plain type `T` always follows the same rules as TypeScript (objects are GC-managed references, primitives are values).

| Modifier | Semantics | After `foo(x)` | Who cleans up? |
|----------|-----------|----------------|----------------|
| `T` | GC-managed (implicit) | `x` still valid | GC |
| `&T` | Borrow (read-only) | `x` still valid | Original owner |
| `&mut T` | Borrow (mutable) | `x` still valid, maybe changed | Original owner |
| `^T` | Ownership transfer | `x` **invalid** | New owner (or GC fallback) |
| `^var T` | Ownership transfer (mutable) | `x` **invalid** | New owner (or GC fallback) |

For the default (`T`), the compiler optimizes automatically:
- Small values are passed by copy (registers)
- Large values are GC-managed references
- Escape analysis promotes heap to stack when safe

### Explicit Ownership (^T)

`T` is not owned by anyone, it is implicitly GC-managed and freed whenever all references to it are gone.
Many people can hold and mutate `T` as long as they like.
`^T` is for when there should only be one owner.
Accordingly, when calling a function with `^T`, the caller gives up ownership of the value to the callee.
After the transfer, the original binding is invalid:

```
function consume(data: ^LargeData) { ... }
const d = LargeData { ... }
consume(^d)    // ownership transferred
print(d.value) // ERROR: use after ownership transfer
```

Use-after-move is an error by default (suppressible to warning).

When a `^T` value goes out of scope without being transferred, it is **dropped**:

```
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    // data dropped here (destructor called)
}
```

Types can implement `Drop` to customize cleanup. This enables RAII patterns.

**Ownership transfer enables:**
- Clear API contracts ("this function takes ownership")
- RAII (files close, locks release, resources clean up)
- Arena integration (transfer into arena)
- Optimization (compiler knows no aliasing)
- Stack allocation without GC overhead

### Explicit Borrowing (&T)

`&T` and `&mut T` are explicit references (pointers) to data. They lower directly to pointer types in MIR:

```
function process(data: &Point) { ... }   // read-only reference
function mutate(data: &mut Point) { ... } // mutable reference
```

Lowers to:
```mir
function @process(v0: ref<Point>) -> void { ... }
function @mutate(v0: ref<Point>) -> void { ... }  ; mutability tracked separately
```

**When to use explicit references:**
- Avoid copying large values when you don't need ownership
- Share read access (`&T`) across multiple callers
- In-place modification (`&mut T`) when the caller retains ownership

For native targets, both `T` and `&T` lower to `ref<T>` in MIR. The difference is source-level semantics and compiler hints.

### Borrow Hints

Destack's `&T` and `&mut T` are **not** Rust-style borrow checking. They're opt-in hints.

**What ownership hints are for:**
- API documentation ("this function borrows, doesn't own")
- Optimization hints (compiler can assume no aliasing for `&mut`)
- Warnings for obvious mistakes (returning `&T` to local variable)

**What ownership hints are not for:**
- A full lifetime system
- A memory safety mechanism (GC handles that)
- Required for correctness

The compiler warns on obvious violations:
```
function bad(): &Point {
    const p = Point { x: 1, y: 2 }
    &p  // warning: returning reference to local variable
}
```

But it won't catch complex lifetime issues that Rust's borrow checker handles. That's fine: GC ensures memory safety regardless. The borrow hints are useful for:
- Performance-critical code where you want to avoid accidental copies
- API design where you want to communicate intent
- Catching simple "oops, returned a reference to a local" bugs

### Arenas and Explicit Ownership

For "systems" programming (compilers, game engines, databases), you need more memory control.

**Arenas:** Allocate many objects, free all at once.
```
const arena = Arena<AstNode>.new()

// allocate into arena - returns &AstNode (reference into arena)
const expr = arena.alloc(BinaryExpr { left, op, right })
const stmt = arena.alloc(IfStmt { cond, then, else_ })

// all allocations freed when arena drops
arena.drop()
```

Arena-allocated values are `&T` references - they live as long as the arena. No individual GC tracking, no per-object overhead.

**No-GC regions:** Forbid GC allocations in performance-critical code.
```
@noManaged
function processFrame(entities: &Entity[]) {
    // compiler error if any GC allocation happens here
    // forces you to use stack, arena, or pre-allocated buffers
}
```

**Ownership transfer:** Use `^T` for explicit ownership transfer.
```
function takeOwnership(node: ^AstNode) {
    // caller gives up ownership, we're responsible for cleanup
}

const node = AstNode { ... }
takeOwnership(^node)  // ownership transferred
print(node.value)     // ERROR: use after ownership transfer
```

**The spectrum in practice:**
```
// Default: GC, simple, like TypeScript
function simple(): User {
    User { name: "Alice" }  // GC-managed, just works
}

// Performance: hints, no unnecessary copies
function transform(data: &LargeData): &Result {
    // borrow input, return reference to field
    &data.result
}

// Systems: arena, no GC
function compileModule(arena: &Arena<Node>, source: string): &Module {
    // all AST nodes allocated in arena
    // zero GC during compilation
}
```

### Copy Elision and Move Semantics

Avoiding unnecessary copies is critical for Rust-level performance.
Lower implements several strategies to minimize data movement.

#### Return Value Optimization (RVO)

When a function returns a locally-constructed value, we allocate directly into the caller's destination:

```
// Source
function makePoint(): Point {
    Point { x: 1, y: 2 }
}
const p = makePoint()
```

Without RVO (copy):
```mir
function @makePoint() -> ref<Point> {
block0:
    v0 = stack.alloc Point
    v1 = iconst 1i32
    v2 = field.set v0, 0, v1     ; set x
    v3 = iconst 2i32
    v4 = field.set v2, 1, v3     ; set y
    return v4
}
; caller then copies result into p
```

With RVO (no copy):
```mir
function @makePoint(v0: ref<Point>) -> void {
block0(v0: ref<Point>):
    v1 = iconst 1i32
    v2 = field.set v0, 0, v1     ; construct directly into dest
    v3 = iconst 2i32
    v4 = field.set v2, 1, v3
    return
}
```

#### Named RVO (NRVO)

Extends RVO to named variables when there's a single return path:

```
function compute(): Data {
    const result = Data { ... }    // named variable
    // ... modify result ...
    result                          // returned
}
// result is constructed directly in caller's destination
```

#### Move Semantics (Explicit Only)

By default, Destack uses **TypeScript semantics**: objects are GC-managed references.
Assignment shares references; variables remain valid after being passed to functions:

```
const a = LargeStruct { ... }
const b = a                    // b shares reference to same object
process(a)                     // a is still valid after this
print(a.property)              // works fine
```

Move semantics only apply with explicit `^T` value types (see above). When passing a value type, the compiler may *move* instead of copy if the source is no longer used:

```
const d = LargeData { ... }
consume(^d)                    // last use of d → moved, not copied
```

**Escape analysis optimization:** Even with reference semantics, the compiler optimizes away unnecessary heap allocations. If a value doesn't escape the function, it can be stack-allocated transparently.

#### Explicit Reference Counting

For scenarios where GC pauses are unacceptable or deterministic destruction is needed,
use the standard library `Rc<T>` and `Arc<T>` types:

```
const shared = new Rc(data)        // explicit RC, single-threaded
const threadSafe = new Arc(data)   // atomic RC for multithreading
const weak = shared.downgrade()    // weak reference, breaks cycles
```

These are explicit opt-in types, not automatic for all objects.
On JS targets, `Rc<T>` and `Arc<T>` compile to plain references (no overhead).

## Closures

Functions that capture variables become closure values:

```
const x = 10
const f = (y: int) => x + y  // captures x
```

Lowers to a closure struct plus a lifted function.
The closure struct (MIR-level) captures the environment:
- `x: int64`

The closure value pairs the function pointer with the environment:
- `fnPtr: FunctionPointer`
- `env: ManagedReference<ClosureEnv>`

The closure body receives `env` as an implicit first parameter.
Closure calls: load `fnPtr` and `env`, call with env prepended to arguments.

---

# Control Flow

Control flow constructs (errors, async, generators) lower to MIR blocks and terminators.
JavaScript's `throw`/`catch` and `async`/`await` are powerful but have runtime costs: exception tables, stack unwinding, state machine overhead.
**Result first error handling**: recoverable errors use `Result<T, E>` (zero cost early returns), while `throw` becomes an abort in native code.
Explicit errors in the type system, panics for bugs only (like Rust).
Async functions lower to state machines, preserving JS `Promise` semantics without the JS runtime overhead.

## Errors and Exceptions

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while `throw` is reserved for unrecoverable panics (bugs).

### Design Philosophy

| Mechanism | Use For | Example |
|-----------|---------|---------|
| `Result<T, E>` | Recoverable errors | Parse failures, file not found, network timeout |
| `throw` | Unrecoverable panics | Assertion failures, invariant violations, bugs |

**Key principle:** Panics indicate bugs, not expected error conditions.
Use `Result` for anything the caller might want to handle.

### Result Types

The standard library provides `Result<T, E>` as a discriminated union.
From Lower's perspective, `Result` is just a tagged union with no special handling.

```
struct Ok<T> { kind: 'ok' = 'ok', value: T }
struct Err<E> { kind: 'err' = 'err', error: E }
newtype Result<T, E> = Ok<T> | Err<E>
```

The `?` operator propagates errors ergonomically:

```
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?;    // propagates Err
    const json = parseJson(text)?;            // propagates Err
    Result.ok(Config.from(json))
}
```

Lowers to early return on error:
```mir
function @readConfig() -> ref<Result<Config, Error>> {
block0:
    v0 = call @readFile(@str_config_json)
    v1 = call @Result_isErr(v0)
    branch v1, block_err, block_ok
block_err:
    return v0                ; propagate error
block_ok:
    v2 = call @Result_unwrap(v0)
    ; ... continue with v2 ...
}
```

### try/catch on Result

The `try`/`catch` syntax works on `Result` types as pattern matching sugar:

```
try {
    const config = readConfig()?
    process(config)
} catch (e) {
    log("Failed to read config:", e)
}
```

This desugars to a `match` on the `Result`. No stack unwinding occurs.
The `catch` block receives the error value from the `Err` variant.

### Panic (throw)

`throw` indicates an unrecoverable error (bug, invariant violation).
Unlike traditional exceptions, panics are not meant to be caught.

```
function assertPositive(n: int) {
    if (n <= 0) {
        throw new Error("invariant violated")  // panic
    }
}
```

**When to use panic:**
- Assertion failures (`assert`, `unreachable`)
- Invariant violations (bugs in the code)
- Unrecoverable states ("this should never happen")

### Target Behavior

| Behavior | Native | JS |
|----------|--------|-----|
| `Result` + `?` | Early return, zero overhead | Early return, zero overhead |
| `try/catch` on `Result` | Pattern match sugar | Pattern match sugar |
| `throw` | **Abort** (no unwinding) | JS throw (catchable) |
| Catching panics | Not possible | Standard try/catch |

**Native targets:** `throw` aborts the process immediately via `Intrinsic::Abort`.
No stack unwinding machinery, no landing pads, no exception tables.
This enables zero-cost error handling on the happy path.

```mir
function @example_panic() -> void {
block0:
    v0 = managed.alloc Error
    ; ... initialize error fields ...
    call @print_panic_message(v0)
    intrinsic.abort()        ; terminate process
    unreachable
}
```

**JS targets:** `throw` behaves as normal JavaScript throw for compatibility.
Code using `throw` for control flow will work in JS but abort in native.

## Coroutines: Async & Generators

Coroutines (async functions, generators) are lowered to state machines.
This is the standard approach used by Rust, C#, and even some modern JavaScript engines (like V8 or JSC), and it fully supports the JS/TS semantics of async/await and generators (i.e., this is just an obvious optimisation).

### State Machine Transformation

Consider an async function that fetches data from a URL:
```
async function fetchData(url: string): Promise<Data> {
    const response = await fetch(url);     // yield point 1
    const json = await response.json();    // yield point 2
    Data.parse(json)
}
```

This lowers to a state machine struct:
```
struct FetchData_StateMachine {
    state: uint32,           // which yield point we're at
    url: string,             // captured parameter
    response?: Response,     // live across yield point 1
    json?: any,              // live across yield point 2
}
```

And a step function that implements the state machine for each suspension point:
```mir
function @fetchData_step(v0: ref<FetchData_StateMachine>, v1: ref<any>) -> ref<PollResult> {
block0(v0: ref<FetchData_StateMachine>, v1: ref<any>):
    v2 = field.get v0, 0           ; load sm.state
    switch v2, block_unreachable, 0 => block_state0, 1 => block_state1, 2 => block_state2

block_state0:
    ; initial: start fetch
    v3 = field.get v0, 1           ; load sm.url
    v4 = call @fetch(v3)
    v5 = iconst 1i32
    v6 = field.set v0, 0, v5       ; sm.state = 1
    v7 = call @Poll_Pending(v4)
    return v7

block_state1:
    ; resumed with response
    v8 = field.set v0, 2, v1       ; sm.response = input
    v9 = field.get v8, 2
    v10 = call @Response_json(v9)
    v11 = iconst 2i32
    v12 = field.set v8, 0, v11     ; sm.state = 2
    v13 = call @Poll_Pending(v10)
    return v13

block_state2:
    ; resumed with json, done
    v14 = field.set v0, 3, v1      ; sm.json = input
    v15 = field.get v14, 3
    v16 = call @Data_parse(v15)
    v17 = call @Poll_Ready(v16)
    return v17

block_unreachable:
    unreachable
}
```

### Generators

Generators work similarly but yield values instead of waiting:

```
function* range(start: int, end: int): Generator<int> {
    for (let i = start; i < end; i++) {
        yield i
    }
}
```

Lowers to:
```
struct Range_StateMachine {
    state: uint32
    start: int
    end: int
    i: int      // loop variable, live across yield
}
```

### Yield Terminator

MIR has a `Yield` terminator for suspension points:
```mir
yield v1, resume: block5, resume_args: [v2]
```

1. Save state (all live locals are in the state machine struct)
2. Return yielded value to caller
3. On resume, jump to resume block with resumed value

### Promise

The default for general targets. Provides full TS `Promise` compatibility:

```
async function fetchData(): Promise<Data> {
    const response = await fetch(url)
    response.json()
}
```

**Event loop:**
- Single-threaded event loop (like Node.js)
- Processes microtasks between macrotasks
- Integrates with OS async I/O (epoll, kqueue, IOCP)

**Microtask queue:**
- Promise resolution callbacks
- `queueMicrotask()` support
- Runs to completion before next macrotask

**Timer integration:**
- `setTimeout`, `setInterval`, `setImmediate`
- High-resolution timers when available

**I/O integration:**
- Async file I/O via thread pool or OS async
- Async networking via non-blocking sockets
- Async DNS resolution

---

# Code Generation

MIR maps to target specific output: native code via Cranelift, WASM, or JS.
MIR is target independent so code generation is mostly mechanical translation.
FFI and calling conventions affect how Destack code interoperates with native libraries.
WASM targets can use WasmGC (browser GC, simpler interop) or WASI (own GC, more portable).
JS codegen happens earlier in the pipeline (directly from DIR), but the concepts here inform what's possible.

## FFI and Calling Conventions

### Calling Conventions

Functions specify their calling convention (in MIR):

```
enum CallingConvention {
    Destack     // internal ABI, can change between versions
    C           // C ABI, for FFI with native libraries
    System      // platform default (Windows: stdcall, Unix: C)
}
```

Default is `Destack`.
Use `C` for `extern` declarations.

### External Functions

Declare-only functions may be marked as `@extern` to indicate that they are defined in another language:

```
@extern("C")
declare function printf(format: &uint8, ...args: any[]): int32
```

Lowers to imported function with `CallingConvention::C` and `Linkage::Import`.

### Exports

Defined functions may be marked as `@export` to indicate that they produce a native export symbol:

```
@export("C")
function add(a: int32, b: int32): int32 { a + b }
```

Generates with `CallingConvention::C` and `Linkage::Export`.

### FFI Error Handling

**Exceptions don't cross FFI boundaries.** This is simple and matches how Rust works.

**C libraries:** Return error codes. Wrap in Result:
```
@extern("C")
declare function fopen(path: &uint8, mode: &uint8): &FILE?

function openFile(path: string): Result<&FILE, IOError> {
    const f = fopen(path.cstr(), "r".cstr())
    if (f == null) { Result.err(IOError.fromErrno()) }
    else { Result.ok(f) }
}
```

**WASM/JS interop:** If importing JS that might throw, wrap it on the JS side:
```js
// JS wrapper returns Result-like object
export function safeParse(text) {
    try { return { ok: true, value: JSON.parse(text) } }
    catch (e) { return { ok: false, error: e.message } }
}
```

**C++ exceptions:** Catch internally, return error codes.

**Destack libraries:** Use Result. Since you have the source (like Rust crates), no FFI boundary.

## DIR -> MIR Mapping

How various DIR constructs are mapped to MIR.

### Declarations

| DIR | MIR |
|-----|-----|
| `Declaration::Function` | `Function` with blocks |
| `Declaration::Class` | `Type::Struct` + methods as functions + vtable |
| `Declaration::Struct` | `Type::Struct` |
| `Declaration::Interface` | `VTable` definition |
| `Declaration::Enum` | `Type::Int` or tagged struct |
| `Declaration::Type` | Resolved during lowering, no MIR artifact |
| `Expression::Let` (const) | `Global` (immutable) or `Local` |
| `Expression::Let` (let) | `Global` (mutable) or `Local` |

### Expressions

Expressions are lowered to MIR blocks with terminators.
Overloaded operators have already been desugared in Elaborate.
Likewise, patterns and match expressions are lowered in Elaborate.

| DIR | MIR |
|-----|-----|
| `ScalarLiteral` | `Const` instruction |
| `Expression::Binary` | `Binary` instruction |
| `Expression::Unary` | `Unary` instruction |
| `Expression::Call` | `Call` or `CallIndirect` |
| `Expression::Member` | `FieldGet` or vtable lookup |
| `Expression::Index` | `ElementGet` |
| `Expression::New` | `ManagedAlloc` + constructor call |
| `Declaration::Function` (arrow) | Closure struct + function |
| `Expression::Await` | `Yield` with Promise resume |
| `Expression::Yield` | `Yield` terminator |
| `Expression::If` | `Branch` terminator |
| `Expression::Match` | `Switch` terminator |
| `Expression::Loop` (while) | Loop with `Branch` |
| `Expression::For` | Loop with `Branch` |
| `Expression::Return` | `Return` terminator |
| `Expression::Throw` | `Intrinsic::Abort` (native) or JS throw (JS target) |
| `Expression::Try` | Pattern match on `Result` (desugared in Elaborate) |

## Target-Specific Considerations

MIR is generated per-target with target-specific decisions and constraints.

### WASM (Browser)

When targeting WASM with browser GC support, we use WasmGC types directly,
letting the browser handle garbage collection.

#### WasmGC Type Mapping

| Destack Type | WasmGC Type | Notes |
|--------------|-------------|-------|
| `boolean` | `i32` | 0 or 1 |
| `int8..int32` | `i32` | Sign-extended as needed |
| `int64` | `i64` | |
| `float32` | `f32` | |
| `float64` | `f64` | |
| `string` | `externref` | JS string interop |
| `T[]` | `(ref (array T))` | WasmGC array |
| `struct S` | `(ref (struct ...))` | WasmGC struct |
| `class C` | `(ref (struct ...))` | With vtable field |
| `T?` | `(ref null ...)` | Nullable reference |
| `any` | `anyref` | Top type |
| Closures | `(ref (struct (field funcref) ...))` | Function + environment |

#### Struct Layout in WasmGC

```
struct Point { x: float32, y: float32 }
```

Becomes:
```wat
(type $Point (struct
    (field $x f32)
    (field $y f32)
))
```

#### Class Layout in WasmGC

```
class Animal { name: string }
```

Becomes:
```wat
(type $Animal (struct
    (field $vtable (ref $Animal_vtable))  ; vtable for virtual dispatch
    (field $name externref)                ; JS string
))

(type $Animal_vtable (struct
    (field $type_id i32)
    (field $speak (ref $func_void_string))
))
```

#### Array Types

Dynamic arrays use WasmGC's array types:
```wat
(type $int_array (array (mut i32)))
```

Fixed-size arrays can use either WasmGC arrays or inline struct fields.

#### Interface Fat Pointers

Structural interface references become structs with two fields:
```wat
(type $Drawable_ref (struct
    (field $object anyref)
    (field $vtable (ref $Drawable_vtable))
))
```

#### JS Interop

- Strings use `externref` pointing to JS strings
- Import JS APIs (DOM, fetch, etc.)
- Export functions for direct JS interop
- Use `extern.convert_any` for crossing the JS/Wasm boundary

### WASM (WASI)

- Own GC implementation (no WasmGC)
- UTF-8 strings
- WASI syscalls for I/O
- More portable, less JS interop

### Native (Cranelift)

- Full control over memory layout
- Tracing GC for managed memory
- System calling convention for FFI
- Direct syscalls via intrinsics