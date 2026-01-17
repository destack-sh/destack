# Interoperability

Destack aims for full **modern TypeScript** compatibility.
Some dynamic JavaScript features are incompatible with ahead-of-time compilation (even when allowing for generous dynamic dispatch and RTTI).
Dynamic features that interfere with AOT compilation (like dynamic imports/eval or shape modification) are restricted or forbidden in native targets (but still fully supported in JS targets).
Fortunately, most modern TS code already avoids such highly dynamic patterns as a best practice.

## Restrictions

JavaScript, as originally designed, is a highly dynamic language with dynamic scopes and little typing guarantees.
Over time, like in many highly dynamic languages, much of the JavaScript community has come around to a restricted, statically typed variant of the language in "modern" TypeScript.

Destack aims to enable native compilation of **modern TypeScript**, _not_ of arbitrary untyped and dynamic JavaScript.
There are other projects that attempt AOT compilation for JavaScript (with varying degrees of success), and such fully untyped or highly dynamic code is explicitly out of scope and cannot be compiled.

### Exception Handling

**On native targets, `throw` aborts the process.**.
There is no stack unwinding, no catching, and misusing `throw` is a compile error.
Instead of classical exceptions, Destack supports `try/catch` and `?` propagation for explicit `Result`-based error handling.

The divergence on exception handling is the most significant semantic difference between Destack and traditional JavaScript/TypeScript:

```ts
// this works in JS/TS:
try {
    throw new Error("oops")
} catch (e) {
    console.log("caught")  // executes
}

// on native Destack: process aborts at throw, catch never runs
try {
    throw new Error("oops")
} catch (e) {
    console.log("abort")  // does not execute!
}
```

Instead, use `Result<T, E>` with the `?` operator for recoverable errors
(or any other type implementing `Try`):

```ds
function readConfig(): Result<Config, Error> {
    const text = readFile("config.json")?;    // propagates error
    Result.ok(parseConfig(text))
}
```

You can still use `try` / `catch` for Result type propagation too, which is useful for manually mapping / wrapping error results or containing the scope of propagation:

```ds
try {
    const config = readConfig();  // propagates to catch block
    console.log(config);
} catch (e) {
    console.error("Failed to read config:", e);
}
```

For JS targets, `throw` works normally for compatibility.
You can still take advantage of Destack's many other features while keeping exceptions around at no extra cost (other than the pre-existing code smell).

Exceptions don't cross FFI boundaries. 
If calling JS that throws (via WASM), wrap it on the JS side to return a Result-like object.
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

### Index Signatures with Static Fields

Types with index signatures (`{ [key: string]: T }`) are supported when the actual fields are statically known:

```ds
interface Config {
    [key: string]: string;  // index signature
    host: string;           // statically known field
    port: string;           // statically known field
}

const config: Config = { host: "localhost", port: "8080" };
config.host              // field access (compile-time offset)
config["host"]           // also field access (literal key)
Object.keys(config)      // returns ["host", "port"] via RTTI
for (const k of Object.keys(config)) { ... }  // iterates known fields
```

**Lowering rules:**
- Type lowers to a struct with the declared fields
- Literal key access (`obj["field"]`) → `field.get`/`field.set`
- `Object.keys/values/entries` → RTTI field list iteration
- `for...in` → same as `Object.keys()` iteration

**Iteration with indexed access:**
```ds
for (const k of Object.keys(config)) {
    console.log(config[k]);  // OK: compiler tracks k comes from Object.keys
}
```
The compiler recognizes this pattern and lowers to RTTI-based field iteration:
```mir
for each field in TypeDescriptor.fields:
    value = intrinsic.rtti_field_get(obj, field)
```

**What's NOT allowed on native (compile error):**
```ds
const key = getUserInput();
config[key]              // ERROR: runtime key on static shape
config[someVariable]     // ERROR: even if variable holds "host"
```

For truly dynamic keys, use `Map<K, V>` or `Record<K, V>` (which aliases to Map on native).

### Choosing Between Index Signatures and Map

Index signatures and Map serve different purposes on native targets. Choose based on your access pattern:

| Pattern | Use Index Signature (struct) | Use Map |
|---------|------------------------------|---------|
| Keys known at compile time | ✓ | |
| Keys determined at runtime | | ✓ |
| Access by literal: `obj["key"]` | ✓ | ✓ |
| Access by variable: `obj[key]` | Only if `key` from `Object.keys()` | ✓ |
| Iterate all entries | ✓ (via RTTI) | ✓ |
| Add/remove keys at runtime | | ✓ |
| O(1) lookup by arbitrary key | | ✓ |

**Examples:**

```ds
// CONFIG: Keys known at compile time → index signature works
interface Config {
    [key: string]: string;
    host: string;
    port: string;
}
const config: Config = { host: "localhost", port: "8080" };
config.host              // ✓ field access
config["port"]           // ✓ literal key
for (const k of Object.keys(config)) {
    console.log(config[k]);  // ✓ iteration pattern
}

// ROUTER: Dynamic lookup at runtime → use Map
const router = new Map<string, (req: Request) => Response>([
    ["/home", handleHome],
    ["/about", handleAbout],
]);
router.get(request.path)  // ✓ O(1) lookup by runtime key

// CACHE: Keys added dynamically → use Map (or Record, which aliases to Map)
const cache: Record<string, User> = {};
cache[userId] = user;       // ✓ Map supports dynamic keys
```

**Rule of thumb:** If you're building a lookup table where keys come from user input, network requests, or other runtime sources, use `Map`. If you're defining a structured object where the fields are part of your API, use a struct (with or without index signature).

### Inferred Static Shapes

Object literals with index signature types infer their shape from construction:

```ds
const obj: { [key: string]: number } = { x: 1, y: 2 };
// Inferred shape: { x: number, y: number }
// Lowers to struct, not Map
obj.x                    // OK: known field
obj["y"]                 // OK: literal key
Object.keys(obj)         // ["x", "y"]
```

If additional properties are added dynamically, use explicit `Map`:
```ds
const obj = new Map<string, number>([["x", 1], ["y", 2]]);
obj.set(dynamicKey, 3);  // OK: Map supports dynamic keys
```

### Computed Property Names

**Native targets:** Computed property keys must be **comptime-known**.
```ds
const key = comptime "dynamicKey";
const obj = { [key]: value };  // OK: key is comptime
const obj = { [runtimeKey]: value };  // ERROR on native
```

**JS targets:** Runtime computed keys are allowed (standard JS behavior).
For dynamic keys on native, use `Map<K, V>` or `Record<K, V>` (which aliases to Map).
This restriction exists because native objects have fixed layouts determined at compile time.
RTTI and structural interfaces with index signatures provide alternatives for dynamic access patterns.

## Semantic Differences

Some TypeScript patterns have different semantics in native vs JS targets, even though we try to preserve the surface area and core semantics.

### Arrays and Indexing

JavaScript arrays allow holes and out of bounds reads yield `undefined`.
Native Destack arrays are dense, `a[i]` is bounds checked, and out of bounds access
triggers the configured check failure.
When targeting JS, the compiler inserts bounds checks so `a[i]` matches native behavior.

### Record Types

In TypeScript, `Record<string, T>` is an object with dynamic string keys:
```ts
const cache: Record<string, User> = {}
cache[id] = user   // dynamic property access
cache.get          // undefined (it's not a Map)
```

For native targets, objects have fixed layouts, so Destack's native libs alias `Record<K, V>` to `Map<K, V>`:

```ds
// source code (works on both targets!)
const cache: Record<string, User> = {};
cache[id] = user;     // reifies to cache.indexSet(id, user)
const u = cache[id];  // reifies to cache.index(id)
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
type JsonValue = null | boolean | number | string | JsonValue[] | Record<string, JsonValue>;
```

This is type-safe without requiring unbounded `any` because JSON has a known, finite set of value types.
Pattern matching on `JsonValue` gives you the concrete type:
```ds
const data = JSON.parse(text);  // JsonValue
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
const user = User.parse(text);  // Result<User, ParseError>
```

Types can also implement the `Serialize` and `Deserialize` interfaces to customize
JSON mapping directly when needed.

**JSON.stringify()** generates serialization code at compile time based on the static type:
```ds
const user: User = ...;
JSON.stringify(user);  // comptime generates User serialization
```

No runtime reflection needed; the compiler knows the type and emits field-by-field serialization, which is type-safe and very efficient.

### Symbol Property Keys

Symbols work as property keys when they're statically known:
```ds
const iter = obj[Symbol.iterator];  // compile-time known, works
obj[Symbol.toStringTag] = "MyType";  // compile-time known, works
```

For dynamic symbol-keyed collections, use `Map<symbol, T>`:
```ds
const sym = Symbol("dynamic");
const map = Map<symbol, string>.new();
map[sym] = "value";  // explicit Map, not object property
```

### Array Methods

Array prototype methods work as expected.
Arrays are (conceptually) monomorphized per element type:

```ds
const nums: int[] = [1, 2, 3];
nums.map(x => x * 2);      // monomorphized: Array_int_map
nums.filter(x => x > 1);   // monomorphized: Array_int_filter
nums.reduce((a, b) => a + b, 0);
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
    count = 0;
    increment = () => { this.count++; };  // this is always Counter instance
}
```
Lowers to a closure that captures `self` in its environment.

**Regular functions/methods** receive `this` as implicit first parameter:
```ds
class Counter {
    count = 0;
    increment() { this.count++; }  // this passed at call site
}
```
Lowers to `@Counter.increment(self)`.

**Standalone functions** have `this = undefined` (strict mode):
```ds
function standalone() { return this; }  // undefined
```

`.call()`, `.apply()`, `.bind()` work by manipulating the implicit `this` parameter:
```ds
fn.call(obj, arg);   // lowers to: fn(obj, arg)
fn.bind(obj);        // lowers to: closure capturing obj as this
```

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
declare function printf(format: &uint8, ...args: unknown[]): int32;
```

Note: Variadic FFI args use `unknown[]` but are passed as raw C values per the calling convention.
This is inherently unsafe; the compiler trusts the format string matches the argument types.

**Exports** use `@export`:
```ds
@export("C")
function add(a: int32, b: int32): int32 { a + b }
```

### FFI Error Handling

Exceptions don't cross FFI boundaries (like Rust).
Wrap C error codes in `Result`; for WASM/JS interop, wrap throwing JS functions on the JS side.

**C function with error code:**
```ds
@extern("C")
declare function open(path: &uint8, flags: int32): int32;

function openFile(path: string): Result<FileHandle, Error> {
    const fd = open(path.cstr(), O_RDONLY);
    if (fd < 0) {
        Result.err(Error.fromErrno(errno()))
    } else {
        Result.ok(FileHandle.new(fd))
    }
}
```

**WASM/JS interop:**
```js
// JS side: wrap throwing function
export function safeFetch(url) {
    try {
        return { ok: true, value: fetch(url) }
    } catch (e) {
        return { ok: false, error: e.message }
    }
}
```

```ds
// Destack side: receives Result-like object
@extern("JS")
declare function safeFetch(url: string): { ok: boolean, value?: Promise<Response>, error?: string };
```
