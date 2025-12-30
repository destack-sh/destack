# Lowering: DIR to MIR

This document describes how Destack's high-level semantic representation (elaborated, canonical DIR) is lowered to machine-level IR (MIR) for native targets (currently via Cranelift).

---

# Overview

## Objectives

The overarching dream is **Rust performance with TypeScript semantics and ergonomics**.
Naturally, these two are in some tension, and we want to enable *up to* Rust performance with some additional constructs while improving modern TS performance without requiring any changes:  

- **Best case (target):** Rust-tier performance (zero-cost abstractions, no GC pauses)
- **Average case (target):** Go-tier performance (efficient GC, good concurrency)
- **Worst case (target):** Competitive with optimized JS runtimes (V8, JSC, SpiderMonkey)

AOT compilation provides predictable performance without warmup, but lots of engineering effort goes into making V8's speculative optimization beat static compilation on some dynamic patterns. 
Our advantage is consistency and control, and, of course, you don't need to ship a JS runtime anymore.

Specifically, Destack lowering is focused on:
1. **TypeScript semantics**: TS and Destack code behaves identically in native*
2. **Comptime**: Full compile-time evaluation
3. **Reflection**: Types-as-values for comptime and runtime reflection
4. **Ownership**: Manual memory or GC as needed
5. **Erasure**: Clean codegen to JS/TS

*Where behavior differs between JS/TS runtimes and native, this difference should be obvious and misue and unexpected results should have loud diagnostics. Perfect semantic equivalence in all scenarios is not required or even possible, since that would require emulating _all_ the non-standard dynamic quirks of common JS runtimes (like optimizer behavior, scheduling, etc.).

### Performance

Destack targets Go-level performance by default and Rust-level performance on-demand.
The compiler relies on a known-good set of optimizations proven out by Go, Rust, Zig, and modern C++ compilers:
- Escape analysis and stack promotion of managed allocations
- Copy elision, return value optimization, and move elimination
- Bounds check elimination with range analysis
- Devirtualization and inlining for class calls
- Monomorphization and specialization control per profile
- Strict borrow mode for `&mut` to enable `noalias` and vectorization
- Explicit SIMD types and intrinsics with scalar fallback
- LTO and PGO for whole-program inlining and layout decisions

SIMD follows a Zig-like model.
Vector lane counts are static parameters and operators are elementwise.
Lower maps vector operations to target SIMD instructions when available.
(If the target lacks support, Lower scalarizes to loops.)

## Pipeline

The mid-end/back-end flow from DIR to MIR (see [compiler/README.md](../README.md)):
```
DIR (elaborated, canonical, profile-dependent)
 │
 ├─→ Execute: run comptime blocks via MIR, patch DIR
 │
 ├─→ Generate/JS: direct JS/TS output (preserves type-erased polymorphism)
 │
 └─→ Lower (THIS DOCUMENT)
      │
      MIR (monomorphized, typed, target-specific)
       │
       ├─→ Verify: validate control flow, types, safety
       ├─→ Optimize: inline, eliminate dead code, etc.
       │
       └─→ Generate
            └─→ Cranelift → native binary (.exe, .dylib)
```

The basic tasks of the lowering pass are:
1. **Monomorphize generics**: each `T` instantiation becomes specialised MIR
2. **Compute layouts**: tag placement, property offsets, struct sizes, alignment
3. **Prepare dispatch**: builtin ops, direct calls, prepare vtables and itabs
4. **Generate type descriptors**: for reflection, `typeOf`, `T.is`, and `instanceof`
5. **Lower control flow**: expressions → blocks with terminators
6. **Allocate locals**: stack slots for variables and temporaries

### Input: Canonical DIR (Comptime Patched)

Lower receives "canonical" typed DIR after Analyze, Elaborate, and Execute (see [analyze/](../analyze/) and [elaborate/](../elaborate/)):
- All desugaring complete (e.g., `+=` → `+` and assign)
- All patterns expanded to decision trees
- All types fully inferred ("Types")
- All overloads resolved ("Resolutions")
- All static parameters resolved to values ("StaticExpression")
- All polymorphic instances created ("Instances")

### Output: Target-Specific MIR

MIR is generated per-target with target-specific decisions:
- Memory layout (LP64, ILP32, etc.)
- Calling conventions (C, System, etc.)
- Alignment requirements

# Interoperability

Destack aims for full **modern TypeScript** compatibility. 
Some dynamic JavaScript features are incompatible with ahead-of-time compilation (even when allowing for generous dynamic dispatch and RTTI). 
Dynamic features that interfere with AOT compilation - like dynamic imports/eval or shape modification - are restricted or forbidden in native targets (but still fully supported in JS targets!).
Fortunately, most modern TS code already avoids such highly dynamic patterns as a best practice.
(And many patterns that seem dynamic can actually be statically analyzed, or the source code easily transformed into something that is more statically known, which usually also improves code quality anyway.)

## Restrictions

JavaScript, as originally designed, is a highly dynamic language with dynamic scopes and little typing guarantees. 
Over time, like in many highly dynamic languages, much of the JavaScript community has come around to a restricted, statically typed variant of the language in "modern" TypeScript.

Destack aims to enable native compilation of **modern TypeScript**, _not_ of arbitrary untyped and dynamic JavaScript.
There are other projects that attempt AOT compilation for JavaScript (with varying degrees of success), and such fully untyped or highly dynamic code is explicitly out of scope and cannot be compiled.

### Exception Handling

**On native targets, `throw` aborts the process.** There is no stack unwinding, no catching.
This is caught by the compiler, and misusing `throw` is a compile error.
Instead, Destack supports `try/catch` and `?` propagation for explicit `Result`-based error handling.
The divergence on exception handling is the most significant semantic difference between Destack and traditional JavaScript/TypeScript:

```ts
// this works in JS/TS:
try {
    throw new Error("oops")
} catch (e) {
    console.log("caught")  // executes
}

// On native Destack: process aborts at throw, catch never runs
```

Instead, use `Result<T, E>` with the `?` operator for recoverable errors:

```ds
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?;    // propagates error
    Result.ok(parseConfig(text))
}
```

You can still use `try` / `catch` for Result type propagation too, which is useful for manually mapping / wrapping error results or containing the scope of propagation:

```ds
try {
    const config = readConfig(); // propagates to catch block
    console.log(config)
} catch (e) {
    console.error("Failed to read config:", e)
}
```

For JS targets, `throw` works normally for compatibility. 
You can still take advantage of Destack's many other features while keeping exceptions around at no extra cost (other than the pre-existing code smell).

Exceptions don't cross FFI boundaries. If calling JS that throws (via WASM), wrap it on the JS side to return a Result-like object. 
See [FFI Error Handling](#ffi-error-handling) for more details.

### Forbidden Features

Fully dynamic features are forbidden in native targets.
No prototype changes, no dynamic object shape modification.

| Feature | Reason | Alternative |
|---------|--------|-------------|
| `eval()` | Arbitrary code execution | Comptime evaluation |
| `with` statement | Dynamic scope modification | Explicit object destructuring |
| `Proxy` | Intercepts all property access | Explicit wrapper types |
| `Reflect` | Runtime metaprogramming | Comptime reflection, RTTI |
| `__proto__` | Prototype chain mutation | Fixed type hierarchy |
| `Object.setPrototypeOf()` | Prototype chain mutation | Fixed type hierarchy |
| Declaration expressions | Runtime type generation | Named declarations (`noDynamicShapes` on JS) |

Some of these features are already discouraged in modern TypeScript (strict mode forbids `with`; `eval` breaks type safety and security), others have solid alternatives as espoused by our standard library.

### Object.prototype Methods

Object prototype methods that depend on a dynamic prototype chain are emulated using RTTI, comptime, or just not available if there is no obvious semantic equivalent:

| Method | Status | Alternative |
|--------|--------|-------------|
| `toString()` | Supported | Via `Display` interface |
| `valueOf()` | Not needed | Implicit coercion discouraged; use explicit conversion |
| `hasOwnProperty(key)` | Limited | Static keys only; use `in` operator or RTTI |
| `constructor` | Aliased | `obj.constructor` becomes `typeOf(obj)` |
| `isPrototypeOf()` | Forbidden | Use `instanceof` (class identity) or `T.is` with RTTI |
| `propertyIsEnumerable()` | Forbidden | Use RTTI reflection |

When RTTI is available, `Object.keys/values/entries` and `hasOwnProperty` can be
lowered for structs/classes by reading their reflected property lists.
For `Record<K, V>` (which aliases to `Map<K, V>` on native targets), these map
to the equivalent `Map` methods.

For `toString()`, types implement the `Display` interface:
```ds
newtype interface Display {
    display(): string
}
```

Primitives and common types have default `Display` implementations.
User types can implement `Display` explicitly or auto-derive it.

### Dynamic Property Access

Static property access (compile-time known keys) works normally:
```ds
point.x           // compile-time field offset
user.name         // compile-time field offset
```

Dynamic property access (i.e., runtime-computed keys) requires special handling:
```ds
obj[computedKey]  // not allowed on structs/classes ("static shapes")
map[computedKey]  // works: Map implements Index<K, V>
record[key]       // works: Record<K, V> aliases to Map<K, V>
```

Types with index signatures (`{ [key: string]: T }`) support dynamic keys (without requiring
`Index/IndexSet` overloads):

- If the key is a compile-time literal and the type has a known field, lower to `field.get/set`
- Otherwise, lower to runtime index intrinsics (`intrinsic.index_get/index_set`)

## Semantic Differences

Some TypeScript patterns have different semantics in native vs JS targets, even though we try to preserve the surface area and core semantics.

### Record Types

In TypeScript, `Record<string, T>` is an object with dynamic string keys:
```ts
const cache: Record<string, User> = {}
cache[id] = user   // dynamic property access
cache.get          // undefined (it's not a Map)
```

For native targets, objects have fixed layouts, so Destack aliases `Record<K, V>` to `Map<K, V>`:

```ds
// source code (works on both targets!)
const cache: Record<string, User> = {}
cache[id] = user   // desugars to cache.indexSet(id, user)
const u = cache[id] // desugars to cache.index(id)
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

### JSON

JSON just works.
Both `JSON.parse()` and `JSON.stringify()` are fully supported, both for statically known types and for dynamic data.

Dynamic JSON parsing with **JSON.parse()** returns `JsonValue`, a closed union of all valid JSON types:
```ds
type JsonValue = null | boolean | number | string | JsonValue[] | Record<string, JsonValue>
```

This is type-safe without requiring unbounded `any` because JSON has a known, finite set of value types.
Pattern matching on `JsonValue` gives you the concrete type:
```ds
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

For typed parsing with validation, use the integrated schema library (which takes advantage of our reflection system):
```ds
const user = User.parse(text)  // Result<User, ParseError>
```

Types can also implement the `Serialize` and `Deserialize` interfaces to customize
JSON mapping directly when needed.

**JSON.stringify()** generates serialization code at compile time based on the static type:
```ds
const user: User = ...
JSON.stringify(user)  // comptime generates User serialization
```

No runtime reflection needed; the compiler knows the type and emits field-by-field serialization, which is type-safe and very efficient.

### Symbol Property Keys

Symbols work as property keys when they're statically known:
```ds
const iter = obj[Symbol.iterator]  // compile-time known, works
obj[Symbol.toStringTag] = "MyType" // compile-time known, works
```

For dynamic symbol-keyed collections, use `Map<symbol, T>`:
```ds
const sym = Symbol("dynamic")
const map = Map<symbol, string>.new()
map[sym] = "value"  // explicit Map, not object property
```

### Array Methods

Array prototype methods work as expected.
Arrays are (conceptually) monomorphized per element type:

```ds
const nums: int[] = [1, 2, 3]
nums.map(x => x * 2)      // monomorphized: Array_int_map
nums.filter(x => x > 1)   // monomorphized: Array_int_filter
nums.reduce((a, b) => a + b, 0)
```

Methods that take callbacks receive closures. 
The closure type is also monomorphized:
- `map<U>((T) => U)` generates code for the specific `T` and `U`
- `sort((T, T) => int)` generates a comparator call for the specific `T`

Mutating methods (`push`, `pop`, `splice`, `sort`) work in-place on the array's backing storage, just like in TypeScript.

### this Binding

JavaScript's `this` binding rules are preserved:

**Arrow functions** capture `this` lexically (from enclosing scope):
```ds
class Counter {
    count = 0
    increment = () => { this.count++ }  // this is always Counter instance
}
```
Lowers to a closure that captures `self` in its environment.

**Regular functions/methods** receive `this` as implicit first parameter:
```ds
class Counter {
    count = 0
    increment() { this.count++ }  // this passed at call site
}
```
Lowers to `@Counter.increment(self)`.

**Standalone functions** have `this = undefined` (strict mode):
```ds
function standalone() { return this }  // undefined
```

`.call()`, `.apply()`, `.bind()` work by manipulating the implicit `this` parameter:
```ds
fn.call(obj, arg)   // lowers to: fn(obj, arg)
fn.bind(obj)        // lowers to: closure capturing obj as this
```

---

# Lowering Model

Destack's type system maps TypeScript's structural, polymorphic types to native representations with concrete, monomorphic layouts.
Each generic instantiation becomes specialized code with known sizes and offsets.
This is "zero-cost generics" (no boxing, no vtables) at the cost of code size, the same tradeoff Rust and C++ make.
TypeScript semantics are preserved: structural compatibility still works, but at the MIR level everything has a concrete type.

## Monomorphization

DIR has Instance-level information from Analysis, but is still polymorphic. 
For native targets, we fully monomorphize these Instances into concrete types with known sizes and offsets.
(Thus, each generic instantiation gets its own specialized MIR logic.)

```ds
function identity<T>(x: T): T { x }

identity<int32>(5)      // generates: identity_int32
identity<string>("hi")  // generates: identity_string
```

Of course, monomorphization trades code size for runtime performance:

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

## Name Mangling

Monomorphized functions need unique, deterministic names for linking.
We use a human-readable scheme (inspired by Rust/Zig):

**Format:** `@<module_path>.<type>.<method>__<type_args>__h<hash>`

**Examples:**
```
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

## Resolution

Resolution tells Lower *which symbol* is being called (or might be called in dynamic resolution) at a call site.
This is determined by Analyze and attached to DIR nodes.
- **Resolution** answers: "Which declaration are we targeting?"
- **Dispatch** answers: "How do we invoke that target at runtime?"

Even with `Resolution::Static`, the target may require vtable dispatch if it's a virtual method on a polymorphic type.
`Resolution::Dynamic` is specifically for *union symbols* where different union variants call different target symbols (but, remember, symbols are polymorphic in DIR so this may be different MIR symbols even for the same DIR symbols).

### Builtin Resolution

Primitive operations on builtin types.
No function call needed; Lower emits MIR instructions directly.

```ds
a + b  // builtin resolution -> int32 + int32
```

Lowers to:
```mir
v2 = iadd v0, v1
```

### Static Resolution

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

**Virtual dispatch** (virtual method on class/interface):
```ds
animal.speak()  // static resolution { target: Animal::speak }, but virtual
```

Lowers to vtable lookup:
```mir
v1 = field.get v0, 0           ; load vtable pointer from object layout
v2 = field.get v1, 2           ; load speak method at vtable slot 2
v3 = call.indirect v2(v0)      ; indirect call through vtable
```

The key: static resolution means we know *which method signature* (Animal::speak), but if it's virtual, the actual implementation depends on the concrete type.

### Dynamic Resolution

Union-based dispatch: different union symbols call different target symbols.
Elaborate has already generated the symbol-dispatch logic; Lower just emits it.

**Union method dispatch:**
```ds
function process(x: Cat | Dog) { 
    // Cat.speak and Dog.speak are different target symbols
    x.speak() 
}
```

Elaborate transforms to a type guard chain (conceptually `instanceof`/`T.is`), Lower then emits:
```mir
type @string = ref<struct { i32, i32, i32 }>
type @ObjectWithVTable = struct { rawptr<void> }

function @process(v0: ref<@ObjectWithVTable>) -> @string {
block0(v0: ref<@ObjectWithVTable>):
    v1 = field.get v0, 0       ; load vtablePtr
    v2 = field.get v1, 0       ; load typeDescriptor from vtable slot 0
    v3 = global.const @Cat_TypeDescriptor
    v4 = icmp_eq v2, v3
    branch v4, block1, block2
block1:
    v5 = call @Cat.speak(v0)
    jump block4(v5)
block2:
    v6 = global.const @Dog_TypeDescriptor
    v7 = icmp_eq v2, v6
    branch v7, block3, block5
block3:
    v8 = call @Dog.speak(v0)
    jump block4(v8)
block5:
    unreachable              ; exhaustive match
block4(v9: @string):
    return v9
}
```

In the `Cat | Dog` case we could also have used an `Animal` interface / base type, and then this would be solved with vtable dispatch instead of dynamic resolution.
(The general principle still applies.)

# Types and Layout

This section defines the concrete runtime representation of types in MIR.
All layouts here are post-monomorphization and target-specific.

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
Default overflow behavior is configurable per-target in build configuration (`overflowChecks`).

**Default (debug mode):** Trap on overflow.
Like Zig and Rust debug builds, arithmetic operations trap on overflow.
This catches bugs early with clear error messages.

**Release mode:** Configurable per-project.
Options: trap (safest), wrap (fastest), or check-and-handle.

**Explicit operators:** Always available regardless of mode.
- `+%`, `-%`, `*%`: wrapping (two's complement wrap)
- `+|`, `-|`, `*|`: saturating (clamp to min/max)

```ds
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
- `null`: Zero/null pointer (0x0)
- `undefined`: Distinguished sentinel value (0x1 or special pattern)

For pointer types, we use **niche optimization** (like Rust's `Option<&T>`).
The null pointer (0x0) is an invalid address for valid objects, so we can use it
as the "none" discriminant without adding a tag byte:

```
T | null  where T is reference type  →  same size as T, null = 0x0
```

This is the same optimization Rust uses for `Option<Box<T>>`, `Option<&T>`, etc.
The "niche" is the invalid bit pattern (null pointer) that we repurpose as a discriminant.

For value types, there's no invalid bit pattern to exploit, so we need a tag:
```
int | null  →  { tag: u8, value: int }  // 2 bytes overhead minimum
```

**Niche optimizations:**

Like Rust, we can exploit invalid bit patterns to save space in type layouts:
- `boolean | null` → use value 2 for null (bool only uses 0 and 1)
- `character | null` → use invalid Unicode scalar values
- Enums with < 256 variants → use unused discriminant values

### BigInt

TypeScript's `bigint` is arbitrary-precision.
For native, we use a tagged pointer representation with small-integer optimization.
(This is similar to what many JS runtimes do internally as well.)

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

```ds
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

The layout is identical across targets except for the payload element type.
UTF-8 payloads use `uint8` data and UTF-16 payloads use `uint16` data.

```
struct string {
    lengthUtf16: uint32,      // UTF-16 code unit count (for TS compatibility)
    lengthBytes: uint32,      // byte length of UTF-8 data
    hash: uint32,             // cached hash (computed lazily)
    data: [uint8; N],         // UTF-8 bytes (inline, variable length)
}
```

GC metadata and type descriptors are stored out of line (see [Managed Object Metadata](#managed-object-metadata)).
For TypeScript semantic compatibility:
- `.length` returns UTF-16 code unit count (not bytes, not codepoints)
- `.charAt(i)` indexes by UTF-16 code units
- ASCII-only strings (common case) use O(1) indexing
On UTF-16 targets, `lengthBytes` caches the UTF-8 byte length and is computed lazily.
The payload encoding is fixed per target configuration.

#### String Literals

String literals are interned at compile time in a read-only data section:

```ds
const s = "hello"   // pointer to static data (never collected)
```

#### Template Literals

```ds
`Hello, ${name}!`
```

Lowers to string concatenation:
```mir
type @string = ref<struct { i32, i32, i32 }>

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
call @Array.push(v0, v4)

; a.length
v5 = field.get v0, 1        ; load length field
```

**Bounds checks:**
Array and slice indexing emits bounds checks by default.
The policy is configured per target (`boundsChecks` in `dsconfig.json`):
- `always`: checks in all builds
- `debug`: checks only when `target.debug` is true (default)
- `never`: no checks (unsafe, fastest)

### Structs and Classes

Both structs and classes are **reference types by default** in Destack (like all TypeScript objects).
Ownership modifiers (`^T`, `&T`) are orthogonal and can force value or reference semantics on either.

| Aspect | struct | class |
|--------|--------|-------|
| Reference identity | No (`===` is compile error) | Yes (`===` compares pointers) |
| Type identity | Via metadata or fat pointer when needed | Always (vtable pointer) |
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
For whole-program compilation, the optimizer already knows what's overridden. 
`final` matters for libraries where downstream users could extend classes.

Both lower to `Type::Struct` with computed property offsets. The key difference is **reference identity**: classes have it (two instances with same data are still different objects), structs don't (two structs with same data are equal). Both can have **type identity** (RTTI) when needed for `instanceof`, `T.is`, or `typeOf`.

#### RTTI and Type Tags

RTTI (runtime type identity) is unified via `TypeDescriptor` pointers.
Classes always store a vtable pointer in the object layout for virtual dispatch.
Vtable slot 0 points at the `TypeDescriptor` for fast `instanceof`, `T.is`, and `typeOf`.
Structs remain headerless and never store a vtable pointer.
Thin-pointer checks on structs recover `TypeDescriptor` from GC metadata when needed.
Interface and `any` values carry `TypeDescriptor` in fat pointers.
Class references are thin pointers, so the vtable pointer must live in the object layout.

GC metadata lookup only applies to managed references.
Non-managed values require explicit tags (union tags or fat pointers) or compile-time type knowledge.

RTTI is only emitted when runtime type checks are possible:
- Used with `instanceof`, `T.is`, or `typeOf` on unknown values
- Stored in `any` or `unknown`
- Used in runtime reflection
- Used in untagged unions that require runtime discrimination

**JS targets:** RTTI-enabled structs/classes emit a non-enumerable symbol property
with their `TypeId` during construction. This keeps objects "plain" for JS semantics
while enabling `instanceof`, `T.is`, and `typeOf` without a global WeakMap.

#### Struct Layout

Structs have no **reference identity** (no `===`).
Structs are always headerless and use metadata or fat pointers for RTTI.

```
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
struct Cat { kind: "cat" = "cat", name: string };
struct Dog { kind: "dog" = "dog", name: string };
type Pet = Cat | Dog;

function greet(pet: Pet) {
    match (pet.kind) {
        "cat" => print("meow")
        "dog" => print("woof")
    }
}
```

String tags are interned to integers (see [String Tag Interning](#string-tag-interning)).

#### Class Layout

Classes always have vtable pointers for virtual dispatch and RTTI:

```ds
class Animal {
    name: string;
    speak(): string { "..." }
}
```

Native layout:
```
struct AnimalLayout {
    vtablePtr: &VTable,      // offset 0, vtable[0] = &Animal_TypeDescriptor
    name: ref<string>,       // offset 8
}
```

Classes have both reference identity (`===` compares pointers) and type identity (via vtable).
Classes always store their vtable pointer because class references are thin pointers and dynamic dispatch is required.
(This is consistent with Java and C++ class objects while keeping struct layouts headerless like Go.)

#### Managed Object Metadata

Managed objects have no per object GC header.
GC metadata is stored out of line in allocator side tables, similar to Go.
Type tags are pointers to TypeDescriptor values, not integer ids.
Null and undefined use niche optimization in the pointer (e.g., 0x0 for null, 0x1 for undefined).

Per span metadata includes:
- Mark bits for GC tracing
- Size class and allocation layout info
- A TypeDescriptor pointer per object for scanning and type queries

Classes still store a vtable pointer in the object for virtual dispatch and fast `instanceof`/`T.is`.
Structs remain headerless and rely on metadata or fat pointers for RTTI.

Tradeoffs:
- Predictable object layouts and smaller per object overhead
- Thin pointer RTTI queries require a metadata lookup

**Explicit value semantics:**
Use `^T` to force value/copy semantics:
```
function process(point: ^Point) {    // ^Point = value type, point is copied
    // modifications don't affect caller
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

```
class Animal {
    name: string
}

class Dog extends Animal {
    breed: string
}
```

Dog's MIR layout:
- offset 0: vtablePtr
- offset 8: name (from Animal)
- offset 16: breed (from Dog)

This ensures a `Dog` pointer can be used where an `Animal` pointer is expected.
Classes use vtables for virtual methods (and non-virtual methods are direct calls).

```
class Animal {
    speak(): string { "..." }
}

class Dog extends Animal {
    speak(): string { "woof" }
}
```

### VTable Layout

Types with virtual methods have a vtable.
The vtable is an array of function pointers, one per virtual method.

**VTable structure (conceptual):**
```
struct VTable {
    typeDescriptor: &TypeDescriptor    // for instanceof, T.is, and typeOf
    destructor: () => void       // cleanup function
    methods: ((...args) => any)[] // virtual method pointers
}
```

**Example vtable layout:**
```
Animal vtable:
  slot 0: typeDescriptor = &Animal_TypeDescriptor
  slot 1: destructor = Animal_drop
  slot 2: speak = Animal.speak

Dog vtable (inherits Animal):
  slot 0: typeDescriptor = &Dog_TypeDescriptor
  slot 1: destructor = Dog_drop
  slot 2: speak = Dog.speak      // overrides Animal::speak
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

**Interface itabs:**
Interfaces use itabs, and interface values carry a fat pointer:
```
{ objectPtr: &Object, itabPtr: &InterfaceItab }
```

Each (Type, Interface) pair has its own itab mapping interface methods to concrete implementations.

Super calls compile to direct calls to parent implementation.

```
class Dog extends Animal {
    speak(): string { super.speak() + " woof" }
}
```

Lowers to:
```mir
type @string = ref<struct { i32, i32, i32 }>
type @Dog = struct { rawptr<void>, rawptr<void> }

function @Dog.speak(v0: ref<@Dog>) -> @string {
block0(v0: ref<@Dog>):
    v1 = call @Animal.speak(v0)   ; direct call, no vtable lookup
    v2 = global.const @str_woof   ; " woof" string literal
    v3 = call @string.concat(v1, v2)
    return v3
}
```

### Getters and Setters

Property accessors lower to method calls.
There is nothing special about them at the MIR level.

```
class Circle {
    #radius: float64

    get area(): float64 { 3.14159 * this.#radius * this.#radius }
    set radius(r: float64) { this.#radius = r }
}

c.area          // call @Circle.get_area(c)
c.radius = 5    // call @Circle.set_radius(c, 5)
```

### Tuples

Tuples lower to anonymous `Type::Struct` with indexed fields.

```
(int, string, bool)  →  Struct { fields: [i64, String, i8] }
```

## Union Types

Destack supports TypeScript's structural unions.

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

TypeScript-style discriminated unions typically use string literals as tags:
```
{ kind: 'loading' } | { kind: 'success', data: T } | { kind: 'error', msg: string }
```

This is a very common pattern in TypeScript, and we can optimize it nicely for native targets.
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
At the source level, `TypeId` is a string like `"@destack-sh/ui/components/button:Button"`.
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

TypeId is a stable string identity for reflection and JS interop.
Native dynamic dispatch does not use TypeId for equality checks.
Native type tags are pointers to TypeDescriptor values.

### Union Representation

Lower chooses union representation based on these rules (in order):

1. **Niche optimization** - All members are nullable references or undefined **and** runtime
   discrimination does not require metadata lookup (or a type tag is already available):
   ```
   string | null       →  ref<string>  (null = 0x0, no tag)
   string | undefined  →  ref<string>  (undefined = 0x1, no tag)
   User | null         →  ref<User>    (null = 0x0, no tag)
   User | undefined    →  ref<User>    (undefined = 0x1, no tag)
   User | null | undefined → ref<User> (null = 0x0, undefined = 0x1, no tag)
   ```
   This requires managed references with at least 2-byte alignment.

2. **Inline tagged** - Total size ≤ 2×pointer_size (16 bytes on 64-bit):
   ```
   int32 | bool        →  { tag: u8, value: int64 }
   int32 | null        →  { tag: u8, value: int32 }
   Point | Line        →  { tag: u8, data: [u8; max(sizeof)] }
   ```

3. **Boxed** - Large or heterogeneous unions, or when runtime discrimination
   needs RTTI but the variants are not tagged:
   ```
   any                 →  { typeDescriptor: &TypeDescriptor, payload: word }
   unknown             →  { typeDescriptor: &TypeDescriptor, payload: word }
   LargeA | LargeB     →  { tag: u8, data: ref<variant> }
   ```

The inline size threshold is fixed per target for ABI stability.
**Owned unions** (`^(A | B)`) prefer inline representation when the variant is known at runtime
without additional RTTI. If RTTI is required for drop, the union is boxed with an explicit tag.
The `typeDescriptor` in boxed unions points at the RTTI descriptor.

### Dynamic Types (any, unknown)

`any` and `unknown` use a shared fat-pointer layout:

```
struct any {
    typeDescriptor: &TypeDescriptor
    payload: word
}
```

The `payload` is a pointer-sized word interpreted by `typeDescriptor`.
`word` is a pointer-sized integer type (u64 on 64-bit, u32 on 32-bit).
Managed references store the object pointer in `payload`.
Small primitives store their bitwise representation directly in `payload`.
Large values are boxed into managed memory and referenced by `payload`.
In `@noManaged` and `@stackOnly` contexts, converting to `any` is a compile error unless the value is already boxed.

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
Type operations execute in the Machine interpreter (which runs MIR), so comptime and runtime share the same representation.
This simplifies the design: there's no separate "comptime type format" vs "runtime type format".

```ds
const PROP_COUNT = comptime User.properties.length;    // → literal 3
const HAS_NAME = comptime User.properties.some(p => p.name == "name")  // → true

if (comptime User.properties.some(p => p.type == string)) {
    // branch selected at compile time, other branch eliminated
}
```

**Runtime:** When type information is needed at runtime, we generate RTTI.
The RTTI table is a static array embedded in the binary.
The native RTTI representation is a compact binary format that maps to the high-level
`Type<T>` API from `language/builtin/core/reflect/type.ds`:

```ds
// high-level API
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
    parentDesc: &TypeDescriptor // for class inheritance (null if none)
    vtablePtr: &VTable          // for virtual dispatch (null if none)
    gcLayoutOffset: uint32      // offset to GC layout bitmap
    gcLayoutWordCount: uint16   // number of words in GC bitmap
    propertiesOffset: uint32    // offset to PropertyDescriptor array
    propertyCount: uint16       // number of properties
    decoratorsOffset: uint32    // offset to DecoratorDescriptor array
    decoratorCount: uint16      // number of decorators
}

struct PropertyDescriptor {
    nameOffset: uint32          // offset into string table
    typeDescriptor: &TypeDescriptor   // TypeDescriptor for property type
    offset: uint32              // byte offset within parent struct
    flags: uint8                // optional, readonly, etc.
}
```

At runtime, when user code accesses `User.properties` or `typeOf(value)`, the `TypeDescriptor`
data is accessed directly.
(Comptime and runtime share the same MIR representation, so no synthesis or conversion step is needed.)
Runtime type tags are pointers to TypeDescriptor values.
Classes store the pointer in vtable slot 0, interface and `any` values carry it in fat pointers,
and thin pointers recover it via GC metadata when needed.

**Lowering Type<T> operations:**

| Source | Comptime | Runtime (if needed) |
|--------|----------|---------------------|
| `User` (in type position) | Type check | N/A |
| `User` (in value position) | Constant TypeDescriptor* | Load from RTTI table |
| `User.name` | Constant "User" | `rtti[user_id].name` |
| `User.properties` | Constant array | Load property descriptors |
| `value instanceof User` | Eliminated if type known | Compare `value.typeDescriptor == &User_TypeDescriptor` |
| `User.is(value)` | Eliminated if type known | Compare `value.typeDescriptor == &User_TypeDescriptor` |
| `typeOf(value)` | Constant if type known | Load `value.typeDescriptor`, return descriptor |

When a value is a thin pointer without an embedded type tag, we get the TypeDescriptor pointer from GC metadata for comparison.

**RTTI generation rules:**
RTTI is only emitted for types that need it at runtime:
- Types used with `instanceof` or `T.is` on unknown values
- Types used with `typeOf()` on unknown values
- Types stored in `any` or `unknown`
- Types with runtime reflection (non-comptime `.properties`, `.name`, etc.)

These are the same rules described above.
If all type operations are comptime, no RTTI overhead appears in the binary.
Dead code elimination removes unused RTTI entries.

---

# Dispatch

Method calls are resolved and dispatched differently based on structural or nominal types (obviously).
TypeScript's duck typing means any object with matching methods can satisfy an interface, which creates some interesting challenges for native codegen.

Class dispatch uses vtables stored in the object layout, like C++ and Java.
Interface dispatch uses itabs carried by fat pointers, like Go.
Union dispatch generates type checking code when a value could be multiple types.

## Dynamic Dispatch

Dynamic dispatch is any situation where the specific function to call is not knowable at compile time, either because the actual symbol is dynamic (union types) or because the symbol is itself a dynamic type that may have multiple implementations.

### Method Calls on Interfaces

Interfaces in Destack can be **structural** (default) or **nominal** (`newtype interface`).
Both use itab-based dispatch with fat pointers, but differ in how they are validated.

#### Structural Interfaces

Structural interfaces require **fat pointers** because the itab layout varies per (Type, Interface) pair:

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
    itabPtr: &InterfaceItab<I>  // pointer to interface itab
}
```

Each (Type, Interface) pair generates its own itab:

```
// Circle as Drawable
const Circle_Drawable_itab: InterfaceItab<Drawable> = {
    typeDescriptor: &Circle_TypeDescriptor,
    draw: @Circle.draw
}

// Rectangle as Drawable
const Rectangle_Drawable_itab: InterfaceItab<Drawable> = {
    typeDescriptor: &Rectangle_TypeDescriptor,
    draw: @Rectangle.draw
}
```

**Interface call lowering:**

Each (Type, Interface) pair gets its own itab with slots assigned in interface method declaration order.
Slot 0 is always `typeDescriptor`, then methods follow.
The compiler generates the itab at compile time, and interface references carry a pointer to the appropriate itab.

```ds
function render(d: Drawable) { d.draw() }
```

Lowers to:
```mir
type @Drawable = struct { rawptr<void>, rawptr<void> }

function @render(v0: rawptr<@Drawable>) -> void {
block0(v0: rawptr<@Drawable>):
    ; v0 is a fat pointer: { objectPtr, itabPtr }
    v1 = field.get v0, 0       ; load objectPtr
    v2 = field.get v0, 1       ; load itabPtr
    v3 = field.get v2, 1       ; load draw method from itab slot 1
    call.indirect v3(v1)       ; call with object as self
    return
}
```

**Interface field access:**

Structural interfaces can include fields as well as methods. 
Field access uses the same fat pointer representation, but the itab also carries a field offset table for the interface's required fields (in declaration order). 
The compiler emits offsets per (Type, Interface) pair.

```ds
interface Named { name: string }
function show(n: Named) { n.name }
```

Lowers to:
```mir
type @Named = struct { rawptr<void>, rawptr<void> }

function @show(v0: rawptr<@Named>) -> rawptr<void> {
block0(v0: rawptr<@Named>):
    v1 = field.get v0, 0       ; load objectPtr
    v2 = field.get v0, 1       ; load itabPtr
    v3 = field.get v2, 1       ; load field offset for name (slot 1)
    v4 = field.get v1, v3      ; load field at offset
    return v4
}
```

**Creating interface references:**

```
const c = Circle { radius: 5.0 }
const d: Drawable = c  // creates fat pointer
```

Lowers to:
```mir
type @Circle = struct { f32 }
type @Drawable = struct { rawptr<void>, rawptr<void> }

function @example() -> rawptr<@Drawable> {
block0:
    v0 = managed.alloc @Circle
    v1 = iconst 5.0f32
    v2 = field.set v0, 0, v1         ; set radius
    ; create fat pointer (inline tuple, no heap allocation)
    v3 = global.const @Circle_Drawable_itab
    v4 = stack.alloc @Drawable       ; allocate fat pointer on stack
    v5 = field.set v4, 0, v0         ; objectPtr
    v6 = field.set v5, 1, v3         ; itabPtr
    return v6
}
```

#### Nominal Interfaces

Nominal interfaces (`newtype interface`) use the same fat pointer representation as structural interfaces.
The difference is typing: nominal interfaces require explicit `implements` declarations.
This makes itabs fully known at compile time and avoids runtime method-set checks.

**Multiple interface implementation:** When a type implements multiple interfaces, methods are
appended to the itab in declaration order. If two interfaces require methods with the same
signature, one implementation satisfies both. If signatures differ, the compiler requires
explicit disambiguation (compile error with guidance).

```
newtype interface Hashable {
    hash(): uint64
}

extension for Circle implements Hashable {
    hash(): uint64 { ... }
}
```
#### Itab Generation

For each (Type, Interface) pair where the type implements the interface:

1. Create a static itab with method pointers in interface declaration order
2. Store the itab as a global constant
3. When creating an interface reference, pair the object with the appropriate itab
4. For `any` or dynamic casts, build and cache the itab at runtime on first use
5. The cache is global per runtime and keyed by `(concrete TypeDescriptor, interface TypeDescriptor)`

**Itab layout:**
```
struct InterfaceItab<I> {
    typeDescriptor: &TypeDescriptor  // for T.is on interface refs
    methods: [FunctionPointer] // one per interface method, in declaration order
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

---

# Memory

Memory allocation and ownership at the MIR level.
TypeScript/JavaScript uses garbage collection with no explicit memory management.
Destack preserves this simplicity by default (GC managed heap allocation), but enables opt in control for performance critical code.
The **GC** is implemnentation is assumed abstractly as "managed allocate" and TS compatible, we assume potential pauses and add some barries.

## Memory Management

Destack supports three allocation modes, controllable via `AllocationMode` per function.
Users can annotate functions with `@noManaged` or `@stackOnly` decorators to enforce these modes:

| Mode | Allocations Allowed | Use Case |
|------|---------------------|----------|
| `Any` | All | Default, full TS compatibility |
| `NoManaged` | `RawAlloc`, `StackAlloc` | Realtime-safe, no GC pauses |
| `StackOnly` | `StackAlloc` | Embedded, deterministic |

### Managed Allocation (GC)

`ManagedAlloc` creates GC-tracked objects.
The runtime provides garbage collection; Lower just emits the allocation instructions.
Allocator selection for native targets is configured per-target (`allocator`).

```mir
type @Point = struct { f32, f32 }

function @alloc_example() -> ref<@Point> {
block0:
    v0 = managed.alloc @Point
    return v0
}
```

**Write barriers:** Lower automatically inserts `Intrinsic::GcWriteBarrier` for all `ManagedReference` field writes.
The runtime uses this for concurrent marking.

**Roots:** Each function has a stack map describing which slots contain managed references.
The GC uses these to find roots during collection.
Managed allocations do not include per object headers.
The allocator side tables store mark bits, size class, and the TypeDescriptor pointer used for scanning.

GC implementation details are target-specific and live in the runtime/codegen layers.

### Raw Allocation

`RawAlloc` / `RawFree` instructions are available for manual memory management (along with the common intrinsics for `memcpy`, `memset`, etc.)

```mir
type @SomeType = struct { i64 }

v0 = raw.alloc @SomeType
; ... use v0 ...
raw.free v0
```

No GC overhead.
Used with ownership annotations (`^T`) for Rust-like semantics.
In debug builds, a tracing allocator (like Zig's) can detect leaks, double-frees, and use-after-free.

### Stack Allocation

`StackAlloc` for frame-local storage:

```mir
type @SomeType = struct { i64 }

v0 = stack.alloc @SomeType
; v0 is automatically freed when function returns
```

No heap allocation.
Used for temporary values, small structs.

## Ownership and Value Semantics

Destack aims to cover the "managedness" spectrum from TS to Go to Rust: implicit GC by default, explicit ownership when needed.
Most code just uses the default, and that should still be plenty fast thanks to real AOT compilation and fixed layouts (more like Go, Java, C#).
Performance-critical code adds these ownership modifiers for manual control.

### Explicit Ownership

To preserve TypeScript semantics, a plain type `T` always follows the same rules as TypeScript (objects are GC-managed references, primitives are values).

| Modifier | Semantics | After `foo(x)` | Who cleans up? |
|----------|-----------|----------------|----------------|
| `T` | GC-managed (implicit) | `x` still valid | GC |
| `&T` | Borrow (read-only) | `x` still valid | Original owner |
| `&mut T` | Borrow (mutable) | `x` still valid, maybe changed | Original owner |
| `^T` | Ownership transfer | `x` **invalid** | New owner (or GC fallback) |
| `^mut T` | Ownership transfer (mutable) | `x` **invalid** | New owner (or GC fallback) |

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

Use-after-move is an error by default (suppressible to warning?).

When a `^T` value reaches its **last proven use** without being transferred, it is **dropped**:

```
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    // data can be dropped before the next statement
}
```

Types can implement `Drop` to customize cleanup. This enables RAII patterns.
Lowering inserts drops at last-use points (non-lexical), including before
control-flow merges and before coroutine suspension when the value is not
used after resume.

```
async function example(flag: bool) {
    const data = ^LargeData { ... }
    if (flag) {
        consume(^data)
        return
    }
    doWork(&data)
    await sleep(10)  // data can be dropped before the await
}
```

**Ownership transfer enables:**
- Clear API contracts ("this function takes ownership")
- RAII (files close, locks release, resources clean up)
- Arena integration (transfer into arena)
- Optimization (compiler knows no aliasing)
- Stack allocation without GC overhead

### Ownership in Fields (Mixing)

Ownership modifiers on fields are allowed but follow strict rules to keep semantics explicit:

- `^T` inside **managed** objects is allowed, but drop is **nondeterministic**.
  If `T: Drop`, the compiler registers a GC finalizer that calls `Drop` on owned fields.
  This is correct but not deterministic; a warning is emitted in strict modes.
- `^T` inside **stack** or **raw** objects is deterministic. Drop order is field order.
- `&T` fields are **disallowed** in managed heap objects by default (no lifetime tracking).
  They are allowed in `@stackOnly` or `@noManaged` contexts, where the lifetime is explicit,
  or via a targeted opt-in for advanced use cases.

### Explicit Borrowing (&T)

`&T` and `&mut T` are explicit references (pointers) to data. They lower directly to pointer types in MIR:

```
function process(data: &Point) { ... }   // read-only reference
function mutate(data: &mut Point) { ... } // mutable reference
```

Lowers to:
```mir
type @Point = struct { f32, f32 }

function @process(v0: ref<@Point>) -> void { ... }
function @mutate(v0: ref<@Point>) -> void { ... }  ; mutability tracked separately
```

**When to use explicit references:**
- Avoid copying large values when you don't need ownership
- Share read access (`&T`) across multiple callers
- In-place modification (`&mut T`) when the caller retains ownership

For native targets, both `T` and `&T` lower to `ref<T>` in MIR. The difference is source-level semantics and compiler hints.
Borrowing a managed object is allowed because the GC is non moving and stack maps treat borrows as roots.

### Borrow Modes

By default, `&T` and `&mut T` are hints with compiler warnings only.
They help document APIs, guide drops, and enable limited optimizations.
Types can implement `Drop` (see "Explicit Ownership" above) for custom cleanup.
Set `borrowMode: "strict"` in `dsconfig.json` to enforce exclusive `&mut` borrows.
Strict mode enables stronger `noalias` optimizations and hard errors on violations.
Strict mode forbids:
- Aliasing `&mut` with any other borrow
- Storing `&mut` inside managed objects
- Holding `&mut` across `await` or generator suspension

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


### Copy Elision and Move Semantics

Avoiding unnecessary copies is critical for "systems-level" performance.
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
type @Point = struct { i32, i32 }

function @makePoint() -> ref<@Point> {
block0:
    v0 = stack.alloc @Point
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
type @Point = struct { i32, i32 }

function @makePoint(v0: ref<@Point>) -> void {
block0(v0: ref<@Point>):
    v1 = iconst 1i32
    v2 = field.set v0, 0, v1     ; construct directly into dest
    v3 = iconst 2i32
    v4 = field.set v2, 1, v3
    return
}
```

#### Named RVO (NRVO)

Extends RVO to named variables when there's a single return path:

```ds
function compute(): Data {
    const result = Data { ... };    // named variable
    // ... modify result ...
    result;                          // returned
}
// result is constructed directly in caller's destination
```

#### Move Semantics

As discussed above, by default, Destack uses **TypeScript semantics**: objects are GC-managed references.
Assignment shares references; variables remain valid after being passed to functions:

```ds
const a = LargeStruct { ... };
const b = a;                    // b shares reference to same object
process(a);                     // a is still valid after this
print(a.property);              // works fine
```

Move semantics only apply with explicit `^T` value types (see above). 
When passing a value type, the compiler may *move* instead of copy if the source is no longer used:

```ds
const d = LargeData { ... };
consume(^d);                    // last use of d → moved, not copied
```

**Escape analysis optimization:** Even with reference semantics, the compiler optimizes away unnecessary heap allocations. If a value doesn't escape the function, it can be stack-allocated transparently.

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

# Concurrency and Memory Model

Destack preserves JS and TS concurrency semantics by default while enabling native level parallelism on supported targets.
The core ideas are a single threaded event loop by default, `Promise` and `async` for concurrency, and `Worker` for parallelism.
Target specific implementations keep surface semantics consistent across JS, WASM, and native targets.

## Threading Model

Default behavior is a single threaded event loop with microtask and macrotask queues.
Native targets support worker threads with the same `Worker` API.
JS targets map to real JS `Worker` instances.
WASM targets map to host specific workers when available.
Native targets map to OS threads with message passing.
Shared memory is explicit and opt-in.

## Memory Model

Shared memory follows JS Atomics semantics.
Atomic operations are sequentially consistent by default and accept explicit orderings when needed.
Non atomic loads and stores have no cross thread ordering guarantees.
Data races on shared non atomic memory are undefined behavior on native targets.
This preserves JS and TS semantics while enabling native performance when code uses atomics.
JS targets follow JS semantics even under data races, with no undefined behavior.

## Atomics and Shared Memory

Shared memory is exposed via `SharedArrayBuffer`-style APIs and MIR atomic intrinsics.

**MIR example, atomic increment with acquire release:**
```mir
function @worker_increment(v0: rawptr<i32>) -> i32 {
block0(v0: rawptr<i32>):
    v1 = iconst 1i32
    v2 = intrinsic.atomic.fetch.add(v0, v1, acq_rel)
    return v2
}
```

**Fence example:**
```mir
function @fence() -> void {
block0:
    intrinsic.atomic.fence(seq_cst)
    return
}
```

## GC and Threads

GC heaps are per worker by default to match JS semantics and avoid sharing mutable GC objects.
GC managed objects are not shared across workers unless explicitly frozen or copied.
Shared memory uses raw pointers or explicit shared buffers.
Native targets may add a shared heap mode in the future, which would require atomic write barriers and a defined cross thread memory model.

## Send and Sync Safety

On native targets, the runtime can enforce that only thread safe values cross worker boundaries.
Types can auto derive `Send` and `Sync` when all fields are `Send` or `Sync`.
GC managed types are not `Send` unless explicitly frozen or copied.
Value types and raw pointers are `Send` by default, and `Sync` when they are immutable or explicitly synchronized.

## Systems Concurrency

Systems workloads rely on worker pools, channels, and atomics.
These are provided in the standard library with comptime ifs for target specific backends.
JS builds use `Worker` and message channels or fall back to single threaded stubs.
Native builds use OS threads, work stealing pools, and lock free primitives.

# Control Flow

Control flow constructs (errors, async, generators) lower to MIR blocks and terminators.
JavaScript's `throw`/`catch` and `async`/`await` are powerful but have runtime costs: exception tables, stack unwinding, state machine overhead.
**Result first error handling**: recoverable errors use `Result<T, E>` (zero cost early returns), while `throw` becomes an abort in native code.
Explicit errors in the type system, panics for bugs only (like Rust).
Async functions lower to state machines, preserving JS `Promise` semantics without the JS runtime overhead.

## Errors and Exceptions

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while `throw` is reserved for unrecoverable panics (bugs).

| Mechanism | Use For | Example |
|-----------|---------|---------|
| `Result<T, E>` | Recoverable errors | Parse failures, file not found, network timeout |
| `throw` | Unrecoverable panics | Assertion failures, invariant violations, bugs |

**Panics indicate bugs**, not expected error conditions.
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

```ds
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?;    // propagates Err
    const json = parseJson(text)?;            // propagates Err
    Result.ok(Config.from(json))
}
```

Lowers to early return on error (note: MIR is post-monomorphization, so `Result<Config, Error>` becomes a concrete monomorphized type like `Result_Config_Error`):
```mir
type @Result_Config_Error = struct { i32, rawptr<void> }

function @readConfig() -> ref<@Result_Config_Error> {
block0:
    v0 = call @readFile(@str_config_json)
    v1 = call @Result.isErr(v0)
    branch v1, block_err, block_ok
block_err:
    return v0                ; propagate error
block_ok:
    v2 = call @Result.unwrap(v0)
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
On native targets, `throw` aborts without unwinding.
Panic policy is configured per target (`panic`), but unwind is reserved for future use.

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
type @Error = struct { i32, rawptr<void> }

function @example_panic() -> void {
block0:
    v0 = managed.alloc @Error
    ; ... initialize error fields ...
    call @print_panic_message(v0)
    intrinsic.abort()        ; terminate process
    unreachable
}
```

**JS targets:** `throw` behaves as normal JavaScript throw for compatibility.
Code using `throw` for control flow will work in JS but abort in native.

## Coroutines: Async & Generators

Coroutines (async functions, generators) are lowered to state machines while preserving both Promise and generator semantics.
This is the standard approach used by Rust, C#, and even some modern JavaScript engines (like V8 or JSC), and it fully supports the existing JS/TS semantics of async/await and generators.

### State Machine Transformation

Consider an async function that fetches data from a URL:
```
async function fetchData(url: string): AsyncResult<Data, Error> {
    const response = await fetch(url)?;     // yield point 1
    const json = await response.json()?;    // yield point 2
    Data.parse(json)
}
```

We need to remember the state of the coroutine at each yield point, so the union of all possible states becomes a state machine struct.
In the above case, we would lower to a state machine struct like this:
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
type @FetchData_StateMachine = struct { i32, rawptr<void>, rawptr<void>, rawptr<void> }
type @any = struct { rawptr<void>, rawptr<void> }
type @PollResult = struct { i32, rawptr<void> }

function @fetchData_step(v0: ref<@FetchData_StateMachine>, v1: ref<@any>) -> ref<@PollResult> {
block0(v0: ref<@FetchData_StateMachine>, v1: ref<@any>):
    v2 = field.get v0, 0           ; load sm.state
    switch v2, block_unreachable, 0 => block_state0, 1 => block_state1, 2 => block_state2

block_state0:
    ; initial: start fetch
    v3 = field.get v0, 1           ; load sm.url
    v4 = call @fetch(v3)
    v5 = iconst 1i32
    v6 = field.set v0, 0, v5       ; sm.state = 1
    v7 = call @Poll.Pending(v4)
    return v7

block_state1:
    ; resumed with response
    v8 = field.set v0, 2, v1       ; sm.response = input
    v9 = field.get v8, 2
    v10 = call @Response.json(v9)
    v11 = iconst 2i32
    v12 = field.set v8, 0, v11     ; sm.state = 2
    v13 = call @Poll.Pending(v10)
    return v13

block_state2:
    ; resumed with json, done
    v14 = field.set v0, 3, v1      ; sm.json = input
    v15 = field.get v14, 3
    v16 = call @Data.parse(v15)
    v17 = call @Poll.Ready(v16)
    return v17

block_unreachable:
    unreachable
}
```

### Generators

Generators work similarly but yield values instead of awaiting promises.
Both use the same state machine transformation and `Yield` terminator in MIR.
The key difference: async functions yield to the executor and await `Promise` resolution,
while generators yield values directly to the caller via `.next()`.

```ds
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

MIR has a `Yield` terminator for suspension points which is used for both async and generator coroutines:
```mir
yield v1, resume: block5, resume_args: [v2]
```

1. Save state (all live locals are in the state machine struct)
2. Return yielded value to caller
3. On resume, jump to resume block with resumed value

### Promise

The `Promise` type provides full TS `Promise` compatibility:

```
async function fetchData(): Promise<Data> {
    const response = await fetch(url)
    response.json()
}
```

The async runtime model is target-specific (like GC), but the general architecture follows
JS semantics for compatibility.

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

MIR is target-independent, so code generation is mostly mechanical translation.
See `language/codegen/` for target-specific backends (Cranelift for native/WASM).

## Debug Info

Lower preserves source information so native code can emit high-quality debug symbols.
This is required for source-level debugging, accurate stack traces, and profiling.
Debug info emission is configured per target (`debugInfo`).

**What Lower records:**
- Source spans on every block and instruction (file, line, column)
- Function debug names (human-readable) plus mangled symbol names for linkage
- Local variable names and lexical scope ranges
- Type debug descriptors (struct/class names, field names, offsets)
- Inline callsite chains for inlined functions

**Codegen output:**
Cranelift consumes this metadata and emits DWARF debug info for native targets.
The MIR itself remains layout-only; debug names are metadata attached to MIR nodes.

## FFI and Calling Conventions

FFI uses the C ABI for interoperability with native libraries.

**Calling conventions:**

| Convention | Use |
|------------|-----|
| `Destack` | Internal ABI (can change between versions) |
| `C` | C ABI for FFI with native libraries |
| `System` | Platform default (Windows: stdcall, Unix: C) |

**External functions** use `@extern`:
```ds
@extern("C")
declare function printf(format: &uint8, ...args: any[]): int32
```

**Exports** use `@export`:
```ds
@export("C")
function add(a: int32, b: int32): int32 { a + b }
```

**FFI error handling:** Exceptions don't cross FFI boundaries (like Rust).
Wrap C error codes in `Result`; for WASM/JS interop, wrap throwing JS functions on the JS side.
