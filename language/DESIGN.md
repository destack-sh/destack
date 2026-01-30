# Destack Language Design

Destack is "TypeScript++" for building correct, optimal, integrated software systems across the _entire_ stack.
Here we describe the motivation and tradeoffs in choosing and extending TypeScript, the '++' parts we added and why, and how it all fits together.
Basically, `.ds` adds features that wouldn't fit into `.ts`, much like `.tsx` or `.svelte` do, but for the entire software stack (from software systems to "systems software").

Of course, other JS and TS-derived languages with some similar ideas and features have tried similar ideas before, and some are even moderately successful (e.g., AssemblyScript, NativeScript (sort of)).
But they all fall short in interoperability, usefulness and - ultimately - adoption.
We think that now is the time to try this again, and have made some different tradeoffs to enable TypeScript to cover a broad spectrum of software systems in a much more optimal and consistent system.

## "TypeScript++"

We're very early in software.
Destack aims to make correct, optimal, integrated full-stack software systems simple and fast to build.
But that requires unifying all the disparate pieces: one language, one type system, one compiler, one linter; ultimately, one way of thinking about code from UI to servers to databases to simulations.

TypeScript is the closest thing we have to a unified software foundation.
Whatever its historical faults, JavaScript - and thus TypeScript - runs everywhere, everyone knows it, _and_ it has a massive ecosystem and install base (i.e., every browser everywhere).

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it _is_ TypeScript(++).
Unlike with C++, our "C" - both JavaScript and TypeScript -- still work perfectly with Destack (on JS/TS targets), and the `++` features are opt-in and complementary (as much as possible).
| Feature | Description | Motivation | Tests |
|---------|-------------|------------|-------|
| [Expressions](#expressions) | Universal expressions, ranges, `match`, patterns, `loop`, `using` | Support pattern matching, compose complex expressions | [expressions/](test/fixtures/specification/expressions/) |
| [Trees](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data | Extend TSX beyond UI to any tree-like structures (prompts, configs, DSLs) | |
| [Annotations](#annotations) | Annotations: decorators and tags (`@`) for _any_ expression | Support rich compile-time metadata and transformations | |
| [Errors](#errors) | `Result`-first error handling with `?` and `??` propagation, no exceptions | Make error handling explicit and zero-cost; exceptions are unpredictable and expensive | |
| [Types](#types) | Type system extensions: newtypes, primitives, structs, tuples | Precise typing, quality of life improvements, performance | [types/](test/fixtures/specification/types/) |
| [Comptime](#comptime) | Compile-time evaluation: precomputation, conditional compilation | Move computation from runtime to compile-time for performance and code generation | |
| [Reflection](#reflection) | Types as values, runtime type descriptors, schema validation | Bridge the gap between static types and runtime needs (validation, serialization) | [declarations/reflection/](test/fixtures/specification/declarations/reflection/) |
| [Dispatch](#dispatch) | Type-dependent dispatch: `extension`s and operator overloading | Add methods to existing types and enable natural mathematical notation | [resolution/](test/fixtures/specification/resolution/) |
| [Ownership](#ownership) | Value ownership / borrowing (`&T`, `^T`) and explicit mutability (`const`/`var`) | Deterministic resource management and memory safety without garbage collection overhead | [types/ownership/](test/fixtures/specification/types/ownership/) |

| Feature | Description | Tests |
|---------|-------------|-------|
| [Expressions](#expressions) | Expression extensions: "as values", ranges, patterns, `loop`, `using` | [expressions/](test/fixtures/specification/expressions/) |
| [Trees](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data | |
| [Annotations](#annotations) | Annotations: decorators and tags (`@`) for _any_ expression | |
| [Errors](#errors) | `Result`-first error handling with `?` and `??` propagation, no exceptions | |
| [Types](#types) | Type system extensions: newtypes, primitives, structs, tuples, constraints | [types/](test/fixtures/specification/types/) |
| [Comptime](#comptime) | Compile-time evaluation: precomputation, conditional compilation | |
| [Reflection](#reflection) | Types as values, runtime type descriptors, schema validation | [declarations/reflection/](test/fixtures/specification/declarations/reflection/) |
| [Dispatch](#dispatch) | Type-dependent dispatch: `extension`s and operator overloading | [resolution/](test/fixtures/specification/resolution/) |
| [Ownership](#ownership) | Value ownership / borrowing (`&T`, `^T`) and explicit mutability (`const`/`var`) | [types/ownership/](test/fixtures/specification/types/ownership/) |
## Expressions

TypeScript inherits JavaScript's expression syntax, and - for the most part - doesn't change it too much to preserve both type-independent emit (i.e., blind erasure) and the familiar JavaScript feeling.
We follow that tradition, but do feel that certain "modern" (in part type-dependent) language features like patterns are significant ergonomic improvements.

### Patterns

Modern `match` with full pattern matching and exhaustiveness checking when the compiler can prove the covered set:

```
match (result) {
    Ok(value) => process(value)
    Err(e) if (e.retryable) => retry()
    Err(e) => fail(e)
}
```

Tagged object patterns accept any object-like type expression.

Exhaustiveness is enforced for finite, statically known sets (enums, literal unions, and discriminated unions with required literal discriminants).
When the compiler cannot prove exhaustiveness, a `_` fallback arm is required.
Irrefutable patterns (like `_`, bindings, or tagged nominal patterns on their exact type) are treated as exhaustive.

`match` is an expression and does not allow `break`.
The match expression type is the union of its case body types.
`switch` keeps TypeScript style fallthrough semantics and remains a statement like expression that yields `void`.

### Statements

In TypeScript, an expression like `if` is a statement that does not produce a value. 
Likewise, for `switch` and most other control flow constructs (except ternary ifs).
In Destack, everything is an expression.
The last non-statement expression (no trailing `;`) becomes the value of the expression.
This enables more ergonomic expressions for complex control flow, and makes complex pattern matching much more ergonomic.

```
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

### If-Let

The "if-let" construct is sugar for matching a value with a refutable pattern as a conditional.
Bindings from the pattern are scoped to the then branch.

```
const result = if let Some(value) = maybe {
    value
} else {
    0
};

if let (x, y) = point {
    print(x + y);
}
```

### Ranges

Range literals for iteration and slicing:

```
for (const i of 0..10) { }      // exclusive
for (const i of 0..=10) { }     // inclusive
```

### Tuples

Explicit tuple syntax with parentheses:

```
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

### Arrays

Arrays are dense and bounds checked by default.
Readonly arrays use `readonly T[]`, and tuples use explicit `()` syntax.
Fixed-size arrays use `T[N]` and are distinct from dynamic `T[]`.

### Loops

Infinite loops with `loop`.
That's it.

```
loop {
    const input = readInput();
    if (input == "quit") { break }
    process(input);
}
```

### Using

`using` is for explicit resource management, mirroring the proposed JS/TS semantics.
Resources are disposed at lexical scope exit in LIFO order, and `await using` calls the async disposer.

```
using file = openFile(path);
await using conn = openConnection();
```

## Trees

TSX syntax generalized for any tree-shaped data:

```
<Prompt>
    <System>Make no mistakes.</System>
    <User>{message}</User>
</Prompt>
```

Tree literals are fully TSX-compatible: copy-paste from `.tsx` files just works.
They work with any tree-compatible type or function, not just React or React-like components.
This enables domain-specific trees for AI prompts, game entities, UI components, and more.

## Annotations

Destack extends decorators (`@`) to work on many more language constructs: declarations, statements, members, parameters, match arms, and more.

```
@deprecated("use newAPI instead")
function oldAPI() {}

@memoize()
function expensive() {}

@unroll()
for (let i = 0; i < 4; i++) {}

// on struct members
struct User {
    @validate(minLength(1))
    name: string,
}

// on function parameters
function process(@nonempty input: string) {}

// on reference types
function kernel(data: @addrspace("shared") &Point) {}

// on match arms
match (result) {
    @cold
    Err(e) => handleError(e),
    Ok(v) => v,
}
```

Decorator behavior depends on what the decorator resolves to:

- **Function**: Transforms the target, `@foo body` desugars to `foo(body)`
- **Newtype**: Compile-time metadata, available for reflection

TypeScript decorators copy-pasted into Destack work as expected.

## Errors

Destack uses **Result-first error handling** inspired by Rust-like languages: recoverable errors use `Result<T, E>`, while `throw` is strongly discouraged and reserved for unrecoverable panics (bugs, invariant violations).
We still _support_ `throw` and classic JS "exceptions" for compatibility with existing JS/TS code, but only on JS targets.

### Result Types

The standard library provides `Result<T, E>` as the primary error handling mechanism.
(This works using operator overloading on the Try `?` operator)

```
function readConfig(path: string): Result<Config, IOError> {
    const text = readFile(path)?;    // propagate errors with ?
    const json = parseJson(text)?;
    Result.ok(Config.from(json))
}
```

The `?` operator propagates errors ergonomically, which is - again - similar to Rust.
When applied to a `Result`, the `?` operator returns early with the error variant if present.
The `??` operator provides a default value instead of propagating:

```
const config = loadConfig() ?? defaultConfig;  // use default on error
```

Both `?` and `??` work via the `Try` interface, which `Result` implements just like any other userland type.
The `??` coalescing operator coalesces nullish values before _and_ after a single `Try` unwrap.
This keeps `Result<T, E> | null` ergonomic without recursive unwrapping.

### Throw and Panic

`throw` is for **unrecoverable errors**: assertion failures, invariant violations, bugs.
Unlike exceptions in Java or Python, panics are not meant to be caught and recovered from.

```
function assertPositive(n: int) {
    if (n <= 0) {
        throw new Error("unrecoverable invariant violated: expected positive")
    }
}
```

**Native targets:** `throw` aborts the process. No stack unwinding, no catching.
This enables "zero-cost" error handling for the common (non-error) path.
**JS targets:** `throw` behaves as normal JavaScript throw for compatibility.

### try-catch

The `try`/`catch` syntax handles exceptions and explicit `Try` propagation:

```
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e: IOError) {
    log("Failed to read config:", e)
}
```

Any type implementing the (nominal) `Try` interface works for try constructs.
Note that just `try` by itself does _not_ implicitly unwrap `Result` values; use `?` or `??` inside the block to propagate `Try` errors into the catch.
Exceptions still propagate into the catch on JS targets (or are rejected by `noExceptions`).

We also support a `match`-style catch expression for comprehensive and ergonomic error handling:

```
try {
    riskyOperationA()?; // Result<T, E1>
    riskyOperationB()?; // Result<T, E2>
} catch match (e) { // e: E1 | E2
    NumericError(x) => Error(`bad number: {x}`))
    FormatError => Error(`bad format {e}`))
}
```

## Types

Destack extends TypeScript's type system with precise primitives, (more) nominal types, more readable constraints (and reflection, but see below).

### Primitives

Precise numeric types beyond TypeScript's `number`:

```
const id: uint64 = 12345;
const balance: float32 = 100.50;
```

Destack keeps `number` as the JS-compatible numeric supertype (aliased to `float64`).
`int`/`uint` and `float` use the compiler's default widths (32-bit ints, 64-bit floats by default).
Pointer-sized integers are spelled `isize` and `usize`.
Explicit float to integer conversions are checked and trap on NaN or out of range values.
Saturating float to integer intrinsics clamp to bounds and map NaN to 0.

### Newtypes

Nominal wrappers that prevent mixing semantically different values without inergonomi and inefficient branding hacks.

```
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

const id = UserId(42);            // wraps scalar
const p = Point(1.0, 2.0);        // wraps tuple
const c = Config({ debug: true }); // wraps object
```

### Structs

Structs are nominal data-oriented value types with fixed layout.
Structs have no identity or inheritance, they're just data with a name.
Structs may _embed_ other structs to compose types, and structs can implement interfaces.

```
struct Point {
    x: float32;
    y: float32;
}
```

#### Structs vs Classes

TypeScript already has classes, and structs are the more machine-friendly way to do many of the same things without any inheritance.

|                    | struct                 | class                            |
| ------------------ | ---------------------- | -------------------------------- |
| Reference identity | ❌ No (`===` is error) | ✅ Yes (`===` compares pointers) |
| Inheritance        | ❌ No (use embedding)  | ✅ Yes (`extends`)               |
| Default passing    | Value                  | Reference                        |
| Default storage    | Inline                 | Managed reference                |
| JS output          | Plain object           | ES6 class                        |

Structs are value types, so `==` compares fields and `===` is forbidden as it would be meaningless.
(Classes are reference types, so `===` compares identity i.e. pointer equality)
Structs also get a default constructor for `new`, and a more explicit and preferred tagged "object" literal form.

```
const p0 = new Point(1, 2); // auto-generated constructor
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
let x: Point = { x, y };        // error: plain object is not Point
p1 == p2;  // true: same data
```

#### Struct Embedding

For composition, structs use embedding instead of inheritance:

```
struct Transform { position: Vec3; rotation: Quat; }
struct Player { ...Transform; health: int; }  // embeds Transform's fields
```

### Constraints

`where` clauses for readable generic constraints:

```
function merge<T: int, U>(): T where (
    U: Comparable<T>
) {}
```

### `this`

Destack supports TypeScript's polymorphic `this` type for instance members, and also allows it in static type positions.
`this` is type-only and resolves to the surrounding receiver or containing type.

### Associated Types

In addition to regular and static members, class-like types can define associated types and associated comptime constants.

#### Associated Types

Structs, classes, and interfaces can declare associated type aliases:

```
struct Cache<K, V> {
    type Entry = CacheEntry<K, V>;  // associated type
    entries: Entry[],
}
```

Associated types are compile-time members of class-shaped declarations, carrying type-level contracts on the owner (i.e., types you can reference statically).
Interfaces can declare required or default associated aliases too, and implementors must then provide concrete definitions.
Associated types can also be generic (GAT-style), including mixed type and comptime value parameters.

#### Associated Comptime Constants

Class-shaped declarations can also expose compile-time associated values with `comptime const`, orthogonal to runtime `static const`.

- `comptime const` is for compile-time association and specialization.
- `static const` is for runtime class or struct members.

```
interface LogStore<Record> {
    comptime const SegmentRows: number = 1024;
    type Segment = Record[this.SegmentRows];
}

class AuditLog implements LogStore<string> {
    comptime const SegmentRows: number = 2048;
}
```

## Comptime

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
Unlike Zig or Rust macros, however, Destack's comptime fills in well-defined **slots** rather than enabling fully arbitrary code generation.
The `comptime` keyword requires that an expression must be evaluated at compile time (otherwise it is a compile error):

```
const LOOKUP_TABLE: uint8[] = comptime {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

```
function factorial(n: int): int {
    if (n <= 1) {
        1
    } else {
        n * factorial(n - 1);
    }
}

const FACT_10 = comptime factorial(10);    // compile time
const dynamicValue = factorial(getUserInput()); // runtime (in this case, at module initialization time)
```

Functions are not marked as "comptime" or "runtime" functions, instead, the call site determines when a function runs.
Static parameters support both type parameters and comptime value parameters.
Value parameters must be marked with `comptime` in the static parameter list.
The `comptime` modifier on parameters requires static evaluation during Analyze.
The `comptime` expression keyword evaluates later during Execute.
Dynamic parameters _may_ be marked `comptime` to require compile-time-known arguments.

Comptime conditions enable branch elimination and, for type relations like `T extends U`, type narrowing:

```
function process<T, Context: CacheContext<T>>(ctx: Context, key: T) {
    if (comptime Context extends EvictableContext<T>) {
        ctx.onEvict(key);  // context is narrowed; branch eliminated if not satisfied
    }
}
```

Comptime blocks can also appear as struct/class members for compile-time assertions:

```
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0 && size <= 65536);
    }
    data: uint8[size],
}
```

Member comptime blocks run once per type instantiation.

### Comptime Execution

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


### Non-Code Modules

Destack also supports importing various file types beyond code modules (much like Node-like runtimes) at "compile time".
Data files are parsed at compile time and type structurally:

```
import config from "./config.json";
// config: { server: { host: string, port: number }, debug: boolean }

config.server.host satisfies string;
config.debug satisfies boolean;
```

Types are inferred from the data:

- `null` → `null`
- `true`/`false` → `boolean`
- Numbers → `number`
- Strings → `string`
- Arrays → `T[]` (union for mixed elements: `(T | U)[]`)
- Objects → `{ key: Type, ... }` (readonly fields)

#### Text Modules

Text files (markdown, CSS, HTML, plain text) import as `string`:

```
import readme from "./README.md";
// readme: string
```

#### Binary Modules

Binary files (images, fonts, wasm, etc.) import as `uint8[]`:

```
import icon from "./icon.png";
// icon: uint8[]
```

#### Import Attributes

Override the default loader with import attributes:

```
import data from "./config.toml" with { type: "json" };  // parse as JSON
import raw from "./data.json" with { type: "text" };     // import as string
import bytes from "./file.txt" with { type: "binary" };  // import as uint8[]
```

The same file with different loaders produces different modules.

## Reflection

In TypeScript, types are (deliberately) erased at runtime.
This was critical for early adoption, but it also means we can't easily perform runtime type checks or any meaningful reflection (without additional libraries or build steps).
Destack supports `Type` as a first-class values to enable reflection with one well-defined system.
(Admittedly, in effect, this _is_ just a "build step", but the entire toolchain is oriented around it.)

### Types as Values

Every type `T` in Destack has a corresponding runtime value of type `Type<T>`:

```
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

Types being values enables patterns that require runtime type information:

```
// generic factory that knows its type parameter
function create<T>(type: Type<T>, data: object): T {
    return type.create(data);
}
const user = create(User, { name: "Alice", age: 30 });

// Runtime type checking
if (User.is(value)) {
    // value is User
}
```

### Decorator Metadata

Decorator information is also accessible at runtime:

```
@deprecated("use newAPI")
function oldAPI() { }

oldAPI.decorators       // [{ name: "deprecated", arguments: ["use newAPI"] }]
```

### Standard Library Schema

The language provides the reflection primitives; the standard library `@destack-sh/schema` provides validation utilities:

1. **Built-in (no imports)**: `Type<T>`, `.name`, `.fields`, `.is()`, decorator access
2. **Standard library**: `parse()`, `safeParse()`

```
import { parse } from "@destack-sh/schema";

const data = await fetchUser();
const user = parse(User, data);    // runtime validation with errors
const user = User.parse(data);     // shorthand (schema extends Type<T>)
```

## Dispatch

TypeScript has parametric polymorphism ("generics") but does not support type-based dispatch as it  historically avoids type-dependent emit all-together.
Destack adds type extensions and real overloading for type-based dispatch and operator overloading (they go hand in hand).

### Overloading

Real function and operator overloading with distinct implementations:

```
function parse(input: string): int32 { 
    return parseInt(input);
}

function parse(input: int32): int32 { 
    return input;
}

extension for Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

For operators, Destack uses **receiver-based dispatch**: `a + b` becomes `a.add(b)`.
Relatedly, to avoid ambiguity, Destack uses **declaration order**: the first matching overload wins.
Overload resolution filters applicable candidates, including static and `comptime` constraints, then selects the first applicable candidate in declaration order.

### Dynamic Resolution

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant.
If all variants resolve to the same symbol, the call is static(ish).
If the symbols differ, the compiler records a dynamic resolution and reifies it into `if (receiver is T)` branches.
This is very convenient, but sometimes undesirable, so there's a warning/lint for it.

Dynamic resolution only applies when every union variant exposes the member.
Arguments must satisfy all candidate signatures, and the resulting type is the union of per-candidate return types after substitutions.
Extension methods participate in member resolution, too.

### Extensions

Destack introduces extensions to add methods and static constants for any nominal type.
These are quite similar to Rust's `impl`:

```
extension for Vector2 {
    magnitude(): float32 { 
        return (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

Extensions require **nominal types**, i.e. types with identity. 
This includes `struct`, `class`, `enum`, `newtype`, and primitive types declared in the prelude (`int32`, `string`, etc.).
Type aliases (`type X = ...`) and inline structural types (`{ x: number }`) cannot be extended, because that would be an ambiguous nightmare.

To extend a structural shape, wrap it in a nominal type:

```
// ❌ Can't extend a type alias or inline shape
type Point = { x: number, y: number };
extension for Point { ... }  // error

// ✅ Use newtype or struct instead
newtype Point = { x: number, y: number };
extension for Point { ... }  // ok
```

Extension visibility follows three simple rules:

- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension for int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils for Date { ... }`).

### Nominal Interfaces

TypeScript interfaces are structural, i.e., any type with matching shape satisfies the interface.
Destack supports that too, of course, but we also support **nominal interfaces** using the `newtype` modifier on `interface` declarations.
(Loosely mirroring how const enums already work).

```
// *structural* interface (standard TypeScript behavior)
interface Drawable {
    draw(): void;
}
const x: Drawable = { draw() {} };  // OK: structural match

// *nominal* interface (requires explicit `implements`)
newtype interface Add<T, R = this> {
    add(other: T): R;
}
```

Nominal interfaces require **explicit `implements`** declarations, i.e., structural compatibility alone doesn't satisfy the constraint.
Nominal interfaces (often represented as `traits`) are used for explicit:

- **Operator interfaces**: `Add`, `Compare`, etc.
- **Marker traits**: `Send`, `Sync`, `Copy`

The `newtype` modifier on `interface` follows the same pattern as `newtype` on type aliases.

## Ownership

TypeScript, like most scripting languages, does not encode ownership in its type system.
Reference types are implicitly GC managed, and value types are copied by default, and that's it.
No raw references, no manual memory, nothing but abstract managedees.
This is fantastic for scripting.

For high performance systems ("systems engineering"), however, explicitly managing memory and ownership is critical.
Destack adds an opt-in explicit ownership and memory management machinery to determines who can use and drop a value, and how to deal with memory.

### Memory Levels

In addition to the default `T` (like `string` or `User`), Destack are four other ownership qualifiers:

```
T            // managed (value or managed reference)
&T           // borrowed (mutable, exclusive reference)
&readonly T  // borrowed (read only reference)
^T           // owned (move-only, mutable)
^readonly T  // owned (move-only, readonly)
```

Raw pointers "opt out" of ownership and are the full responsibility of the programmer:

```
*T           // raw pointer (mutable, unsafe)
*readonly T  // raw pointer (readonly, unsafe)
```

Memory safety comes down to more than just ownership and borrowing, but nonetheless it is often helpful to explicitly enforce no aliasing and other ownership rules.
To that end, the readonly variants enable optional Rust-like ownership rules:
- `&T` and `&readonly T` are safe borrows verified by the borrow check pass.
- Assigning through `&readonly T` is invalid, mutation requires `&T`.
- Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
- Dropping or freeing a value while it is borrowed is always an error.
- Conflicting borrows and invalidating stores are errors (or warnings in lenient mode).

Sometimes it is useful to manage pointers directly, for which we support `*T` and `*readonly T`:
- `*T` and `*readonly T` are unsafe pointers with no borrow tracking.
- Raw pointers may be null or dangling and allow pointer arithmetic.
- Converting between borrowed references and raw pointers is always explicit.
- Raw pointers do not imply ownership or drop behavior.

### Address Spaces

References can target explicit address spaces for native and accelerated targets.
The default address space is `generic`, which maps to the target's normal memory.
Address spaces are spelled with `addrspace(name)` or `addrspace(7)` on a reference:

```
ref<raw addrspace(shared) i32>
ref<raw addrspace(7) i32>
```

Address space changes are explicit (and lower to the `addrspace.cast` intrinsic).
The VM provides deterministic host side models for non generic address spaces when available.

### Ownership Transfers

When you transfer ownership with `^T`, reusing the original value is an error:

```
const node = AstNode { ... }
consume(^node)    // ownership transferred
print(node.value) // ERROR: use after ownership transfer
```

`^T` is the owning handle type.
Passing a `^T` by value transfers ownership to the callee.
Use `^expr` to convert a value `T` into an owning handle `^T`.
(If you already have `^T`, pass it directly, no `^expr` needed.)

`^T` values are dropped at their last proven use (non lexical), not just at end of scope:

```
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&readonly data)            // borrow it
    log("done")                       // data can be dropped before this line
}
```

This enables RAII patterns even for unmanaged resources.

### Allocation and Drop

Ownership modifiers determine how values are allocated and cleaned up:

| Source                          | Allocation      | Cleanup    | Semantics                       |
| ------------------------------- | --------------- | ---------- | ------------------------------- |
| `T` (plain)                     | `managed.alloc` | GC         | GC handles everything           |
| `^T` (owned)                    | `raw.alloc`     | `raw.drop` | dispose + deallocate            |
| `&T` (borrow)                   | none            | none       | points to someone else's memory |

For manual memory without destructors (FFI, low-level code), use `raw.free` directly instead of `raw.drop`.
Destack 

**Drop behavior:**

- `Drop` is a marker interface that opts a type into last use cleanup.
- Types that implement `Drop` must also implement `Symbol.dispose`, which is invoked by the drop glue.
- `using` always calls `Symbol.dispose`, even without `Drop`.
- Owned values are dropped at their last proven use unless `using` is specified.
- `using` bindings drop at scope end and cannot be moved.
- `^T` controls ownership transfer and move semantics.
- `using` controls drop timing and does not imply ownership.
- Combine `using` with `^T` for deterministic cleanup of owned values.


### Nested Ownership

Ownership is determined at the **usage site**, not the type definition.
A struct can contain `^T` fields regardless of how the struct itself is used:

```
struct Container { data: ^Data }

const a: Container = ...           // managed container
const b: ^Container = ...          // owned container
```

When the container is owned (`^Container`), drops are **deterministic**: fields drop in reverse declaration order, then the container drops.
When the container is managed (`Container`), drops are **nondeterministic**: GC finalizers handle owned fields when the container is collected.
A warning is emitted in strict mode for `^T` fields in managed types.

### Ownership Conversions

Ownership conversions are explicit, except for borrows inserted at reference boundaries.
Implicit ownership conversions only create borrows and never transfer ownership.

Implicit conversions:

- `T` → `&T` or `&readonly T` when a reference type is required and the value is addressable
- `^T` → `&T` or `&readonly T` when a reference type is required
- `&T` → `&readonly T` to reborrow as shared

Explicit conversions:

- `T` ↔ `^T` require explicit ownership operators or helper calls (use `^expr` for `T` → `^T`)
- `&T` → `T` requires `Copy` or an explicit clone
- `&T` → `^T` requires an explicit clone and ownership transfer
- `*T` conversions require explicit unsafe operations

Elaborate inserts implicit borrows before other implicit casts.

### Returning References

Functions can return borrowed references (`&T`).
Borrowed returns use lifetime inference and `@lifetime` annotations to track which inputs they borrow from.
In strict mode, returning a borrow that may outlive its origin is an error.
In lenient mode, the same situation produces a warning.

```
function get(container: &Container): &Item {
    &container.item  // ok: borrowing from input
}

function bad(): &Point {
    const p = Point { x: 1, y: 2 }
    &p  // WARNING: returning reference to local variable
}
```

### Lifetime Annotations

When returning references or aggregates containing references, the compiler needs to know which input parameters the return value borrows from. 
This is usually inferred:

- Single `&readonly T` parameter → return borrows from it
- `&readonly this` receiver → return borrows from receiver
- Multiple `&readonly T` parameters → conservative (borrows from all)

When inference is too conservative, use `@lifetime` to be explicit:

```
// explicit: only borrows from 'a', not 'b'
function first(a: &readonly string, b: &readonly string): @lifetime("a") &readonly string {
    return a;
}

// may borrow from either
function pick(a: &readonly string, b: &readonly string): @lifetime("a", "b") &readonly string {
    if (cond) { return a; }
    return b;
}

// static: borrows from static data only
function constant(): @lifetime("static") &string {
    return &"hello";
}

// struct with borrowed field
function makeTokenizer(source: &string): @lifetime("source") Tokenizer {
    return Tokenizer { source, pos: 0 };
}
```

The compiler verifies annotations—returning something that doesn't borrow from the declared parameters is an error. This avoids Rust-style `<'a>` annotations while still enabling precise borrow tracking where needed.
