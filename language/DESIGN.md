# Destack Language Design

> **The Destack language is "TypeScript++" for building correct, optimal, integrated software systems.**
>
> This document describes the motivation and tradeoffs in choosing TypeScript and why we added what.

Destack is designed as a superset of TypeScript. 
Destack also supports compiling to JS/TS targets, and works with regular JS/TS dependencies when they fit Destack's strict typed model.
So, if you don't need or want any additional features you can ignore the "++" part of Destack entirely, write completely standard `.ts` and `.tsx` files, and just stop reading right here.
For most use cases, most of the time, the "++" is happily out of sight and out of mind.

Destack adds features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do, but for truly full stack software systems.
We support full `TSX` syntax, and modern `TS` code just works, because the Destack language ("TS++") is a superset of modern _TypeScript_.
To enable truly universal progrmaming with TypeScript, even in high performance ("systems") use cases, we support some _additional_ stuff like manual memory management features.

Invariably, when starting with an existing language as feature rich as modern TypeScript, any _new_ additions risk becoming unpredictably combinatorial in their complexity (hello C++).
We tried hard to keep the actual net new concepts to the minimal set required to express all the missing things we needed, while also filling some gaps we experienced in the language that TypeScript cannot address directly (mostly due to its commitment to type-free emit).

## "TypeScript++"

We're very early in software.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
We cannot confidently build the next generation of software without unifying all the disparate pieces: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to a unified software foundation today that _could_ conceivably express all software (because in many ways, it already is, albeit suboptimally).
Unlike Python (and Rust and Go and insert your favorite language), the TypeScript ecosystem also has a good answer to rich frontends *and* a very strong "already runs everywhere" story because browsers are the most ubiquituous execution platform.

So, TypeScript runs everywhere, everyone knows it, and it has a massive ecosystem.
If you can compile to JS/TS, and behave like TS, you get a "new" language that doesn't actually feel new, but more like TSX or Svelte.
Then, because modern TypeScript is very close to a fully AOT-compilable language, we can build a new toolchain completely free of JS runtimes and "legacy" code while staying true to the behavior most developers already know well. 

Thus, wherever Destack looks like TypeScript - e.g., `interface`, `class`, `async`/`await`, objects, templates, generics, types, everything! - it behaves like TypeScript, because it _is_ TypeScript.
Unlike with C++, our "C" - both JavaScript/TypeScript -- still work with Destack, and the `++` features are opt-in and complementary.

| Feature | What | Why |
|---------|-------------|-----|
| [**Expressions**](#expressions) | Expression extensions: "as values", patterns, `loop`, `using` | Better ergonomics |
| [**Trees**](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data | TSX is great |
| [**Annotations**](#annotations) | Annotations: decorators and tags (`@`) for _any_ expression | Annotate metadata |\| [Errors](#errors) | `Result`-first error handling with `?` and `??` | |
| [**Types**](#types) | Type system extensions: newtypes, primitives, structs, tuples, constraints | Soundness, memory, precision |
| [**Comptime**](#comptime) | Compile-time evaluation: precomputation, conditional compilation | Metaprogramming |
| [**Reflection**](#reflection) | Types as values, runtime type descriptors, schema validation | Metaprogramming |
| [**Dispatch**](#dispatch) | Type-dependent dispatch: `extension`s and operator overloading | Better ergonomics |
| [**Ownership**](#ownership) | Value ownership, borrowing (`&T`, `^T`) and explicit deep mutability | Systems programming |

## Expressions

In TypeScript, control flow exprsesions like `if` are a statement, and you need a ternary or temporary to get a value out.
To enable more ergonomic data flow and particularly better pattern matching capabilities, Destack also supports statements as expressions ("everything is an expression").
Like in similar languages, the last non-statement expression (no trailing `;`) becomes the value of the expression.

```ds
const result = if (condition) {
    computeA()
} else if (condition) {
    computeB()
} else {
    computeB()
};

function add(a: int, b: int): int {
    a + b // implicit return
}
```

Building on statements-as-expressions, if-let expressions enable nice sugar for matching a value with a refutable pattern in a conditional.
Bindings from the pattern are then available in the positive branch, and this composes with TS flow typing as you would expect.

```ds
const result = if let Some(value) = maybe {
    value
} else {
    0
};

if let (x, y) = point {
    print(x + y);
}
```

### Tuples

Explicit tuple syntax with parentheses:

```ds
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

### Arrays

Arrays like `T[]` (or `Array<T>`) are dense, homogenous and bounds checked by default with no holes allowed.
Thus, accessing into `T[]` just gives you a straight `T` always (no `T | undefined`), because out of bounds and holes are both forbidden.
Like in JS/TS, dynamic `Array` grow automatically like you would expect.

In addition to dynamic arrays, Destack also provides fixed-size arrays with `T[N]`.
Because TypeScript already uses `T[N]` for indexed access, Destack honors that behavior when indexed access is admissible, and we have to use `N as comptime` to force fixed-size array construction in ambiguous cases.
(We also provide a `FixedArray<T, comptime N>` as an explicit alias for `T[N as comptime]`.)

### Patterns

Modern `match` with full pattern matching and exhaustiveness checking:

```ds
match (result /* Result<T, E> */ {
    { kind: 'ok', value } => process(value)
    { kind: 'err', error } if (isRetryable(error)) => retry()
    { kind: 'err', error } => fail(error)
}
```

As you would expect, the type of a match expression is the union of its case body types.

### Loops

Infinite loops with `loop`.

```ds
loop {
    const input = readInput();
    if (input == "quit") { 
        break;
    }
    process(input);
}
```

### Using

`using` is explicit scoped cleanup scheduling with TS-shaped surface syntax.
Resources are cleaned up at lexical scope exit in LIFO order, and `await using` runs async cleanup when required.
The same cleanup capabilities also power ownership based destruction.
Affine owned values may therefore be cleaned up earlier, at their last proven use, even without `using`.
`using` does not replace ownership.
It chooses one cleanup scope explicitly.

```ds
using file = openFile(path);
await using conn = openConnection();
```

## Trees (TSX)

Destack generalizes TSX syntax for any tree-shaped data:

```ds
// Wall.ds
<Wall id={1}>
    <Block name="foo" color={Color.RED} />
    <Block name="bar" color={Color.BLUE} />
</Wall>

// Prompt.ds
<Prompt>
    <System>You are a helpful assistant.</System>
    <User>{userMessage}</User>
</Prompt>

// Level.ds
<Level difficulty={3}>
    <Player position={spawn} />
    {enemies.map(e => <Enemy {...e} />)}
</Level>
```

Destack's tree literals work with any tree-compatible type, not just UI component systems, and not just any _single_ JSX/TSX-style per project. 
Because we have real type analysis you can mix and match.
Types can opt into custom tree tag behavior by implementing the `TreeTag` interface, and custom intrinsic types (lowercase tags like `<div>`) are programmable via `TreeTagBuilder`. 

## Annotations

Destack extends decorators (`@`) to work on many more language constructs: declarations, statements, members, parameters, match arms, and more.

```ds
@deprecated("use newAPI instead")
function oldAPI() { }

@memoize
function expensive() { }

@unroll
for (let i = 0; i < 4; i++) { }

// on struct members
struct User {
    @validate(minLength(1))
    name: string,
}

// on function parameters
function process(@nonempty input: string) { }

// on reference types
function kernel(data: @space("shared") &Point) { }

// on match arms
match (result) {
    @cold
    Err(e) => handleError(e),
    Ok(v) => v,
}
```

Decorator behavior depends on what the decorator resolves to:
- **Function**: Transforms the target, `@foo body` desugars to `foo(body)`
- **Newtype**: Compile-time metadata, available for reflection but stripped from output

TypeScript decorators copy-pasted into Destack work as expected.

## Errors

Destack strongly encourages **Result-first error handling** inspired by Rust: recoverable errors use `Result<T, E>`, while exceptions exist for compatibility, interop, and migration.
`Result<T, E>` with `?` and `??` remains the preferred everyday style.

### Result Types

The standard library provides `Result<T, E>` as the primary error handling mechanism:

```ds
function readConfig(path: string): Result<Config, IOError> {
    const text = readFile(path)?;    // propagate errors with ?
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

The `?` operator propagates errors ergonomically using the builtin `Try` operator, similar to Rust.
When applied to the builtin `Result` type, `?` returns early with the error value if present.

TypeScript's `??` coalescing operator then supports a convenient default value for the failure case:

```ds
const config = loadConfig() ?? defaultConfig;  // use default on error
```

### throw and native exceptions

Detack also supports exceptions and `throw` for compatibility with classical JS/TS and other exception-oriented ecosystems like Java and C#.
Destack still prefers `Result` for ordinary recoverable errors, especially in performance-critical code.

```ds
function assertPositive(n: int) {
    if (n <= 0) {
        throw new Error("invariant violated: expected positive")
    }
}
```

### try/catch with Result and exceptions

The `try`/`catch` syntax handles both exceptions and explicit `Try` propagation:

```ds
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e: IOError) {
    log("Failed to read config:", e)
}
```

The example uses `Result`, but any type implementing `Try` behaves the same:
 - Note that `try` does not implicitly unwrap `Result` values
 - Use `?` or `??` inside the block to propagate `Try` errors into the catch
 - When a `?` is inside a `try` with a catch, `Try.fromError` is not required

Thrown exceptions propagate into the catch in the usual way when exceptions are enabled (i.e., no `no_exceptions`).
As usual, a `try` expression must include a `catch` or `finally` block.

```ds
try {
    riskyOperationA()?; // -> Result<void, AError>
    riskyOperationB()?; // -> Result<void, BError>
} catch match (e /* AError | BError */) {
    NumericError(x) => Error(`bad number: ${x}`))
    FormatError => Error(`bad format ${e}`))
    _ => Error(`unknown error: ${e}`))
}
```

## Types

Destack extends TypeScript's type system with precise primitives, nominal types ("newtypes"), ergonomic constraints, and some additional features.

### Primitives

Precise numeric types beyond TypeScript's `number`:

```ds
const id: uint64 = 12345;
const balance: float32 = 100.50;
```

Destack keeps `number` as the JS-compatible numeric supertype (aliased to `float64`).
`int` and `uint` are fixed-width aliases for `int64` and `uint64`.
`float` defaults to `float64`.
Pointer-sized integers are spelled `isize` and `usize`.

### Newtypes

Nominal wrappers that prevent mixing semantically different values:

```ds
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

const id = UserId(42);            // wraps scalar
const p = Point(1.0, 2.0);        // wraps tuple
const c = Config({ debug: true }); // wraps object
```

### Structs

Structs are data-oriented value types with fixed layout without reference identity or any inheritance; they are just data with a name.
Structs may embed other structs to compose types, and structs can implement interfaces.

```ds
struct Point {
    x: float32;
    y: float32;
}
```

#### Structs vs Classes

| | struct | class |
|---|---|---|
| Reference identity | No (`===` is error) | Yes (`===` compares managed identity) |
| Inheritance | No (use embedding) | Yes (`extends`) |
| Default passing | Value | Reference |
| Default storage | Inline | Managed reference |
| JS output | Plain object | ES6 class |

Classes are reference types, so `===` compares managed object identity as usual.
Structs are value types, so `==` compares fields and `===` will just error (at compile time).

```ds
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
p1 == p2;  // true: same data

const e1 = new Entity(1);
const e2 = new Entity(1);
e1 == e2;  // false: different instances
```

#### Structs Are Nominal

Structs are nominal (like newtypes), so they must be explicitly constructed:
Prefer `Point { ... }` for structs and reserve `new Point(...)` for compatibility with TS-style call sites.

```ds
let x: Point = Point { x, y };  // ok
let x: Point = { x, y };        // ERROR: plain object is not Point
```

For composition, structs use embedding instead of inheritance:

```ds
struct Transform { position: Vec3; rotation: Quat; }
struct Player { ...Transform; health: int; }  // embeds Transform's fields
```

#### Associated Types

Structs, classes, and interfaces can declare associated type aliases:

```ds
struct Cache<K, V> {
    type Entry = CacheEntry<K, V>;  // associated type
    entries: Entry[],
}
```

Associated types are resolved at compile time and can reference static parameters.
Associated projection uses the same member lookup and substitution model as value members.

### Constraints

`where` clauses for readable generic constraints:

```ds
function merge<T: int, U>(): T where (
    U: Comparable<T>
) { }
```

### The `this` Type

Destack supports TypeScript's polymorphic `this` type for instance members, and also allows it in static type positions.
`this` is type-only and resolves to the surrounding receiver or containing type.

## Comptime

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
The `comptime` keyword requires that an expression must be evaluated at compile time (otherwise it is a compile error):

```ds
const LOOKUP_TABLE: uint8[] = comptime {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

```ds
function factorial(n: int): int {
    if (n <= 1) { 1 } else { n * factorial(n - 1) }
}

const FACT_10 = comptime factorial(10);    // compile time
const dynamicValue = factorial(getUserInput()); // runtime (in this case, at module initialization time)
```

Functions are not marked explicitly as either "comptime" or "runtime" functions, instead, the call site determines when a function runs.

Static parameters support both type parameters and comptime value parameters.
Value parameters must be marked with `comptime` in the static parameter list.
The `comptime` modifier on parameters requires static evaluation during Analyze.
The `comptime` expression keyword evaluates later during Execute.
Dynamic parameters _may_ be marked `comptime` to require compile-time-known arguments.

Comptime conditions enable branch elimination and, for type relations like `T extends U`, type narrowing:

```ds
function process<T, Context: CacheContext<T>>(ctx: Context, key: T) {
    if (comptime Context extends EvictableContext<T>) {
        ctx.onEvict(key);  // context is narrowed; branch eliminated if not satisfied
    }
}
```

Comptime blocks can also appear as struct/class members for compile-time assertions:

```ds
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0 && size <= 65536);
    }
    data: uint8[size],
}
```

Member comptime blocks run once per type instantiation.

### Execution Model

Destack distinguishes **static execution** and **comptime execution**:

- **Static execution** is a small, closed-form subset that can be evaluated during Analyze.
  This includes literals, arithmetic on literals, known constants, and other syntax that can be folded without
  executing user code. These are required for static parameters and type-level arguments.
- **Comptime execution** evaluates `comptime` expressions and blocks by running MIR in the
  VM interpreter during the Execute phase. Results are written back into the program
  as constants and dead branches are eliminated.

Static execution must not depend on full comptime execution.
This avoids dependency cycles between static expressions and comptime execution.
Full comptime evaluation happens after monomorphization and lowering, with full type information available.

## Reflection

In TypeScript, types are - by design - erased at runtime.
This was critical for early adoption, but it also means you can't easily perform runtime type checks or any meaningful reflection (without additional libraries or build steps).
Destack supports `Type` as a first-class values to enable reflection with one well-defined system.

### Types are Values

Every type `T` in Destack has a corresponding runtime value of type `Type<T>`:

```ds
struct User {
    name: string;
    age: uint;
}

// User in type position: the type
let u: User = User { name: "Alice", age: 30 };

// User in value position: the type descriptor
const UserType = User;              // UserType: Type<User>
UserType.name                       // "User"
UserType.fields                     // [{ name: "name", type: string }, ...]
```

### Decorator Metadata

Decorator information is accessible at runtime:

```ds
@deprecated("use newAPI")
function oldAPI() { }

oldAPI.decorators       // [{ name: "deprecated", arguments: ["use newAPI"] }]
```

## Dispatch

TypeScript has parametric polymorphism ("generics") but does not support type-based dispatch (by design).
Destack adds type extensions and real overloading for type-based dispatch and operator overloading.

### Extensions

Destack introduces extensions to add methods and static constants for any _nominal_ type:

```ds
extension of Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions require **nominal types**—types with identity. 
This includes `struct`, `class`, `enum`, `newtype`, and primitive types declared in the prelude (`int32`, `string`, etc.).
Type aliases (`type X = ...`) and inline structural types (`{ x: number }`) cannot be extended (because that would be very unpredicable)

To extend a structural shape, wrap it in a nominal type:

```ds
type Point = { x: number, y: number };

extension of Point { ... }  // ERROR

newtype Point = { x: number, y: number };
// works - extend newtype / struct / class / ..
extension of Point { ... }  // ok
```

Extension visiblity is basically as you would expect:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension of int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils of Date { ... }`).

### Nominal Interfaces

TypeScript interfaces are structural, i.e., any type with matching shape satisfies the interface.
This is usually what we want, but sometimes nominality is required for a contract, and in those cases the TS ecosystem usually uses branding symbols.

Destack adds real **nominal interfaces** using the `newtype` modifier on `interface` declarations (spiritually related to `const enum`, this is basically syntactic sugar around `newtype`):

```ds
// structural interface (standard TypeScript behavior)
interface Drawable {
    draw(): void;
}
const x: Drawable = { draw() {} };  // OK: structural match

// nominal interface (requires explicit `implements`)
newtype interface Add<T, R = this> {
    add(other: T): R;
}
```

Nominal interfaces require **explicit `implements`** declarations.
Structural compatibility alone doesn't satisfy the constraint.
Nominal interfaces (often represented as `traits`) are used for:

- **Operator interfaces**: `Add`, `Compare`, etc.
- **Capability traits**: `Send`, `Sync`, `Copy`, `Clone`

The `newtype` modifier on `interface` follows the same pattern as `newtype` on type aliases, making a `newtype interface` more like a nominal trait in other languages.

```ds
// structural interface: requirements only
interface Drawable {
    draw(): void;
}

// nominal interface: defaults allowed
newtype interface Print<T> {
    print() {
        // do nothing by default
    }
}
```

### Overloading

Real function and operator overloading with distinct implementations:

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

extension of Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

For operators, Destack uses **receiver-based dispatch**: `a + b` becomes `a.add(b)`.
Following TS, to avoid ambiguity, Destack uses **declaration order**, i.e., the first matching overload wins.

### Dynamic Resolution

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant.
If all variants resolve to the same symbol, the call is static.
If the symbols differ, the compiler records a dynamic resolution and reifies it into `if (receiver is Type)` branches.

Dynamic resolution only applies when every union variant exposes the member.
Arguments must satisfy all candidate signatures, and the resulting type is the union of per-candidate return types after substitutions.
Extension methods participate in member resolution, too.

## Ownership

TypeScript does not encode "ownership" in its type system: all reference types are implicitly GC managed, and all value types are copied by default.
This is convenient, but sometimes we want to take direct ownership of memory, whether for better control and performance, or just to express and enforcve invariants in the code.
Destack adds explicit, optional modifiers for controlling memory ownership and placement inspired by Rust and Mojo's ownership models with `^T` as the "owned" signifier.

Further, as TS already has a strong notion of local memory as the implicit memory model, we also support explicit shared memory model as part of a generalised, explicit notion of "place" local to a worker, shared across workers, or another address space.
On the shared heap side, this is basically a generalisation around `SharedArrayBuffer`-like semantics.
As with ownership, most of the time, developers don't need to think about placement, but it is very useful in certain situations.

| Form | Ownership | Region | Place | Liveness | MIR shape | Value |
|------|-----------|--------|-------|---------------|-----------|-------|
| `T` | managed | none | ambient | keeps the referent alive | `ref<T, managed, space(local)>` | managed heap handle |
| `shared T` | managed | none | shared | keeps the referent alive | `ref<T, managed, space(shared)>` | managed shared handle |
| `&T` | borrowed | inferred or explicit | ambient | requires liveness | `ref<T, borrowed, space(X)>` | semantic borrow or projection |
| `&shared T` | borrowed | inferred or explicit | shared | requires liveness | `ref<T, borrowed, space(shared)>` | semantic shared borrow or projection |
| `^T` | owned | none | ambient | owns the referent | `ref<T, owned, space(X)>` | owned heap handle |
| `^shared T` | owned | none | shared | owns the referent | `ref<T, owned, space(shared)>` | owned shared heap handle |
| `*T` | raw | none | ambient | does not keep anything alive | `ref<T, raw, space(X)>` | unsafe raw typed pointer |
| `*shared T` | raw | none | shared | does not keep anything alive | `ref<T, raw, space(shared)>` | unsafe shared raw typed pointer |

Managed and owned values both live on the heap.

Heap storage is traced whenever `T` may contain references.

Raw storage is never traced.

### Relations

The rules for who can point into what mostly follow from the fact that references must always be valid, and shared memory should not point into local memory.
(And raw pointers are your own dangerous business.)

| From \ To | `T` | `shared T` | `&T` | `&shared T` | `^T` | `^shared T` | `*T` | `*shared T` |
|-----------|-----|------------|------|-------------|------|-------------|------|-------------|
| `T` | - | no | yes | no | no | no | explicit unsafe | no |
| `shared T` | no | - | no | yes | no | no | no | explicit unsafe |
| `&T` | no | no | - | no | no | no | explicit unsafe | no |
| `&shared T` | no | no | no | -- | no | no | no | explicit unsafe |
| `^T` | no | no | yes | no | - | no | explicit unsafe | no |
| `^shared T` | no | no | no | yes | no | - | no | explicit unsafe |
| `*T` | no | no | unsafe checked reborrow | no | no | no | - | no |
| `*shared T` | no | no | no | unsafe checked reborrow | no | no | no | - |

### Bindings

Because Destack inherits the JS/TS "Worker" model for isolation and memory ownership, module-scoped constants are owned by each _Worker_ and not actually process-global as they would be in many other languages.
For actually shared process-global globals, the binding itself can be declared as `shared`.

| Form | Binding cell | Value place | Meaning |
|----------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

### Allocation And Destruction

The primary typed construction path is `new`.

`new` allocates heap storage, initializes a `T`, and produces the ownership form required by the destination type and the base type's affinity.

`new` is destination-typed.

The expected type decides whether construction produces a managed or owned value, with affine base types preferring owned forms in unconstrained positions.

There is no second primary typed allocation surface alongside `new`.

`raw.alloc` and `raw.free` are reserved for true raw storage only.

`drop` ends ownership of a `^T`, runs destruction, and releases owned heap storage.

`dispose` and `dispose.async` are resource cleanup protocols rather than allocation primitives.

### Regions

A region is one compiler-known lifetime relation for one borrow.
Explicit region spelling is only needed when a signature must relate returned borrows to input borrows.

```ds
&T                 // surface syntax
Borrowed<T, _>     // normalized form

@lifetime("a") &T
Borrowed<T, "a">
```

Declarations that store borrowed fields are implicitly region generic.
Owned fields make the enclosing type affine.
Affine storage-bearing positions default to owned storage rather than implicit heap storage.
Borrowed fields make the enclosing type region generic.

Managed objects may contain owned fields.

That integration is required so ownership composes with ordinary managed programming rather than creating a second disjoint language.

Types containing owned fields are affine and therefore do not silently copy.

Ordinary borrows should not cross `await`, suspension, or worker transfer boundaries at first.

### Typing

Internally, Destack normalizes ownership and place into one type-addressable `Form<T, O, S, R>`.
That lets ordinary TS-style type algebra talk about base type, ownership, place, and borrow region directly, which makes for some very convenient conditional and mapped type algebra.

Traceability is derived from the base type and layout metadata rather than from ownership itself.

| Surface spelling | Algebraic spelling |
|------------------|--------------------|
| `T` | `Managed<T>` in storage bearing positions |
| `&T` | `Borrowed<T, _>` |
| `^T` | `Owned<T>` |
| `*T` | `Raw<T>` |
| `shared T` | `Shared<T>` |
| `@space("shared") T` | `WithSpace<T, "shared">` |

The core kernel is:

```ds
type Form<T, O = "managed", S = "local", R = never> = ...

type BaseOf<T> = ...
type OwnershipOf<T> = ...
type SpaceOf<T> = ...
type RegionOf<T> = ...

type OwnershipOr<T, D> = ...
type SpaceOr<T, D> = ...

type Managed<T> = ...
type Borrowed<T, R> = ...
type Owned<T> = ...
type Raw<T> = ...
type Shared<T> = ...

type WithBase<Q, T> = ...
type WithOwnership<Q, O> = ...
type WithSpace<Q, S> = ...
type WithRegion<Q, R> = ...
```

## Module Imports

Destack supports importing various file types beyond code modules, following Bun's approach to asset imports.

### Data Modules (JSON, TOML, YAML)

Data files are parsed at compile time and typed structurally:

```json:config.json
{ 
    server: { 
        host: "127.0.0.1", 
        port: 8080 
    }, 
    debug: false
}
```

```ds:main.ds
import config from "./config.json";

config.server.host satisfies string;
config.server.port satisfies number;
config.debug satisfies boolean;
```

Types are inferred from the data:
- `null` → `null`
- `true`/`false` → `boolean`
- Numbers → `number`
- Strings → `string`
- Arrays → `T[]` (union for mixed elements: `(T | U)[]`)
- Objects → `{ key: Type, ... }` (readonly fields)

### Text Modules

Text files (markdown, CSS, HTML, plain text) import as `string`:

```ds
import README from "./README.md";
README satisfies string;
```

### Binary Modules

Binary files (images, fonts, wasm, etc.) import as `uint8[]`:

```ds
import icon from "./icon.png";
icon satisfies uint8[];
```

### Import Attributes

Override the default loader with import attributes:

```ds
import dataJson from "./data.toml" with { type: "json" };  // parse as JSON
import dataRaw from "./data.json" with { type: "text" };     // import as string
import dataBytes from "./file.txt" with { type: "binary" };  // import as uint8[]
```

Supported `type` loaders are `json`, `toml`, `yaml`, `text`, `binary`, and `base64`.
(The same file with different loaders produces different modules, of course.)

## Compatibility

**Destack aims for 100% compatibility with _modern_ TypeScript.**
To be completely fair, this is a little sneaky, because we get to decide what "modern" means - but really, it just means that much of the deprcated TS legacy stuff is unsupported, and most _runtime dynamic_ JS features are deliberately out of scope (`Function`, `eval`, `prototype` modification, etc.).
See [COMPATIBILITY.md](COMPATIBILITY.md) for more depth on this.

Syntax-wise, for `.ds` files, a few obscure syntax patterns work differently due to built-in TSX support and additional typing features:

| Pattern | `.ts` | `.tsx` | `.ds` |
|---------|-------|--------|-------|
| `<T>() => ...` | Generic arrow | Ambiguous (use `<T,>`) | Ambiguous (use `<T,>`) |
| `(a, b, c)` | Comma operator | Comma operator | Tuple literal |

Fortunately, these patterns already rarely appear in production code:
- The **generic arrow** ambiguity already exists in `.tsx` files—`.ds` inherits this since it supports TSX syntax natively. The workaround (`<T,>`) is standard practice in TSX codebases.
- The **comma operator** is mostly seen in minified code or obscure one-liners. Destack uses `()` for tuples instead, which is more explicit and composes better with the type system than TypeScript's `[T, U]` array syntax.

Destack does not and will not support:
- **Flow**: We support TypeScript only.
- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript. Non-strict ("sloppy mode") behaviors like duplicate function declarations or `yield` as an identifier are not supported. This aligns with how TypeScript modules work (always strict) and modern best practices.
- **Declaration expressions (native targets)**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with ahead-of-time compilation. Use named declarations instead. On JS targets, enable `noDynamicShapes` for portability.
- **XML namespace resolution**: Destack does not implement XML `xmlns` namespace binding semantics.
  Namespaced tree tags like `<svg:path />` are treated as intrinsic string tag names (`"svg:path"`).
