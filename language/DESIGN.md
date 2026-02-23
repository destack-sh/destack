# Destack Language Design

> **Destack is "TypeScript++" for building correct, optimal, integrated software systems.**
>
> This document describes the motivation and tradeoffs in choosing TypeScript and why we added what.

Destack adds features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do, but for whole software systems including high performance ("systems") use cases.

Of course, other JS/TS-derived languages with similar features have tried this before, and some are moderately successful (e.g., AssemblyScript, NativeScript (sort of)).
But they all fall short in interoperability, usefulness and - ultimately - adoption.
We feel that now is the time to try this again, and have made some different tradeoffs to enable TypeScript to cover many more usage scenarios.

See [SPECIFICATION](SPECIFICATION.md) for the fine-grained language definition.
See [COMPATIBILITY](COMPATIBILITY.md) for interoperability details.

## "TypeScript++"

We're very early in software.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
We cannot build the next generation of software without unifying all the disparate pieces: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to a unified software foundation today.
JavaScript runs everywhere, everyone knows it, and it has a massive ecosystem and install base (i.e., every browser everywhere).
Unlike Python, the TypeScript ecosystem also has a good answer to rich frontends *and* strict modern TypeScript is a much more optimizable language (as evidenced by V8 and JSC coming within touching distance of Go and C# in some scenarios).

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`, objects, templates, generics, types), it behaves like TypeScript.
Unlike with C++, our "C" - both JavaScript/TypeScript -- still work with Destack (on JS/TS targets), and the `++` features are opt-in and complementary.

| Feature | Description | Tests |
|---------|-------------|-------|
| [Expressions](#expressions) | Expression extensions: "as values", patterns, `loop`, `using` | [expressions/](test/fixtures/specification/expressions/) |
| [Trees](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data | |
| [Annotations](#annotations) | Annotations: decorators and tags (`@`) for _any_ expression | |
| [Errors](#errors) | `Result`-first error handling with `?` and `??` propagation, no exceptions | |
| [Types](#types) | Type system extensions: newtypes, primitives, structs, tuples, constraints | [types/](test/fixtures/specification/types/) |
| [Comptime](#comptime) | Compile-time evaluation: precomputation, conditional compilation | |
| [Reflection](#reflection) | Types as values, runtime type descriptors, schema validation | [declarations/reflection/](test/fixtures/specification/declarations/reflection/) |
| [Dispatch](#dispatch) | Type-dependent dispatch: `extension`s and operator overloading | [resolution/](test/fixtures/specification/resolution/) |
| [Ownership](#ownership) | Value ownership / borrowing (`&T`, `^T`) and explicit mutability (`const`/`var`) | [types/ownership/](test/fixtures/specification/types/ownership/) |

## Expressions

In TypeScript, `if` is a statement, and you need a ternary or temporary to get a value out.
Same with `switch` and most other control flow (except ternary ifs).
In Destack, everything is an expression.
The last non-statement expression (no trailing `;`) becomes the value of the expression.
This enables more ergonomic expressions for complex control flow.

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

If let is sugar for matching a value with a pattern in a conditional.
Bindings from the pattern are scoped to the then branch.
If let without else yields void.

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

Arrays are dense and bounds checked by default.
Readonly arrays use `readonly T[]`, and tuples use explicit `()` syntax.
Fixed-size arrays use `T[N]` and are distinct from dynamic `T[]`.
Because TypeScript uses `T[N]` for indexed access, Destack keeps that behavior when indexed access is admissible.
Use `N as comptime` to force fixed-size array construction in ambiguous cases, including numeric literals like `string[4 as comptime]`.
(We also provide a `FixedArray<T, comptime N>` as an explicit alias for `T[N as comptime]`.)

### Patterns

Modern `match` with full pattern matching and exhaustiveness checking:

```ds
match (result) {
    Ok(value) => process(value)
    Err(e) if (e.retryable) => retry()
    Err(e) => fail(e)
}
```

Tagged object patterns accept any object-like type expression.

`match` is an expression and does not allow `break`.
The match expression type is the union of its case body types.
`switch` keeps TypeScript style fallthrough semantics and remains a statement like expression that yields `void`.

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

`using` is explicit protocol based resource disposal, mirroring JS/TS semantics.
Resources are disposed at lexical scope exit in LIFO order, and `await using` calls the async disposer when available.
`using` is independent from ownership: use `using` for `Disposable` or `AsyncDisposable`, and use `^T` when you need single owner memory lifetime control.

```ds
using file = openFile(path);
await using conn = openConnection();
```

## Trees

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
function kernel(data: @addrspace("shared") &Point) { }

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

Destack uses **Result-first error handling** inspired by Rust: recoverable errors use `Result<T, E>`, while `throw` is reserved for unrecoverable panics (bugs, invariant violations).
We still support `throw` and classic JS exceptions for compatibility with existing JS/TS code, but only on JS targets.

### Result Types

The standard library provides `Result<T, E>` as the primary error handling mechanism:

```ds
function readConfig(path: string): Result<Config, IOError> {
    const text = readFile(path)?;    // propagate errors with ?
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

The `?` operator propagates errors ergonomically, similar to Rust.
When applied to a `Result`, it returns early with the error if present.
The `??` operator provides a default value instead of propagating:

```ds
const config = loadConfig() ?? defaultConfig;  // use default on error
```

Both operators work via the `Try` interface, which `Result` implements.
`Try.branch()` returns a structural `TryBranch<T, E>` shape.
Structural `TryBranch` compatibility is based on object shapes, not nominal structs or newtypes.
`Try.fromError` is required when a `?` propagates out of the enclosing function.
The `??` operator coalesces nullish values before and after a single `Try` unwrap.
This keeps `Result<T, E> | null` ergonomic without recursive unwrapping.

### Panic (throw)

`throw` is for **unrecoverable errors**: assertion failures, invariant violations, bugs.
Unlike exceptions in Java or Python, panics are not meant to be caught and recovered from.

```ds
function assertPositive(n: int) {
    if (n <= 0) {
        throw new Error("invariant violated: expected positive")
    }
}
```

**Native targets:** `throw` aborts the process. No stack unwinding, no catching.
This enables zero-cost error handling for the common (non-error) path.

**JS targets:** `throw` behaves as normal JavaScript throw for compatibility.

### try/catch with Result and exceptions

The `try`/`catch` syntax handles exceptions and explicit `Try` propagation:

```ds
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e: IOError) {
    log("Failed to read config:", e)
}
```

The example uses `Result`, but any type implementing `Try` behaves the same.
`try` does not implicitly unwrap `Result` values.
Use `?` or `??` inside the block to propagate `Try` errors into the catch.
When a `?` is inside a `try` with a catch, `Try.fromError` is not required.
Exceptions still propagate into the catch on JS targets, or are rejected by `no_exceptions` on native.
A try expression must include a catch or finally block.

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
`int`/`uint` and `float` use the compiler's default widths (32-bit ints, 64-bit floats by default).
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

Structs are data-oriented value types with fixed layout.
Structs have no identity or inheritance, just data with a name.
Structs may embed other structs to compose types, and structs can implement interfaces.
Struct values can be boxed when a reference is required, which allocates managed storage without changing the struct type.
Boxing is compiler inserted and does not introduce a surface `Box<T>` type.
Boxing copies the value into managed storage, and repeated boxing creates distinct reference identities.
Structural object types remain reference types, even when written as type aliases.
Type aliases inherit the semantics of the underlying type.
Struct declarations require a name and cannot be anonymous.

```ds
struct Point {
    x: float32;
    y: float32;
}
```

#### Structs vs Classes

| | struct | class |
|---|---|---|
| Reference identity | ❌ No (`===` is error) | ✅ Yes (`===` compares pointers) |
| Inheritance | ❌ No (use embedding) | ✅ Yes (`extends`) |
| Default passing | Value | Reference |
| Default storage | Inline | Managed reference |
| JS output | Plain object | ES6 class |

Structs are value types, so `==` compares fields and `===` is not defined.
Classes are reference types, so `===` compares identity.
Ownership modifiers (`^T`, `&T`) describe access and lifetime without changing identity semantics.
Struct values may still be heap allocated by escape analysis, but the semantics remain value based.

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
See [Associated Types](SPECIFICATION.md#associated-types) for full details.

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
Unlike Zig or Rust macros, however, Destack's comptime fills in well-defined **slots** rather than enabling fully arbitrary code generation.
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

Functions are not marked as "comptime" or "runtime" functions, instead, the call site determines when a function runs.
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

Destack introduces extensions to add methods and static constants for any nominal type:

```ds
extension for Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions require **nominal types**—types with identity. This includes `struct`, `class`, `enum`, `newtype`, and primitive types declared in the prelude (`int32`, `string`, etc.).
Type aliases (`type X = ...`) and inline structural types (`{ x: number }`) cannot be extended.

To extend a structural shape, wrap it in a nominal type:

```ds
type Point = { x: number, y: number };

extension for Point { ... }  // ERROR: can't extend a type alias or inline shape

newtype Point = { x: number, y: number };
// works - extend newtype / struct / class / ..
extension for Point { ... }  // ok
```

Extensions let you add methods to any nominal type: classes, structs, enums, newtypes, even primitives and foreign types without modifying the original definition.
The members of an extension are visible as you would expect:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension for int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils for Date { ... }`).

### Nominal Interfaces

TypeScript interfaces are structural, i.e., any type with matching shape satisfies the interface.
Destack adds **nominal interfaces** using the `newtype` modifier on `interface` declarations:

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
- **Marker traits**: `Send`, `Sync`, `Copy`

The `newtype` modifier on `interface` follows the same pattern as `newtype` on type aliases.

### Overloading

Real function and operator overloading with distinct implementations:

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

extension for Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

For operators, Destack uses **receiver-based dispatch**: `a + b` becomes `a.add(b)`.
Relatedly, to avoid ambiguity, Destack uses **declaration order**: the first matching overload wins.
Overload resolution filters applicable candidates, including static and `comptime` constraints, then selects the first applicable candidate in declaration order.
Overload order is defined at the declaring module and is forwarded unchanged across module boundaries.

### Dynamic Resolution

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant.
If all variants resolve to the same symbol, the call is static.
If the symbols differ, the compiler records a dynamic resolution and reifies it into `if (receiver is Type)` branches.

Dynamic resolution only applies when every union variant exposes the member.
Arguments must satisfy all candidate signatures, and the resulting type is the union of per-candidate return types after substitutions.
Extension methods participate in member resolution, too.

## Ownership

TypeScript does not encode "ownership" in its type system: all reference types are implicitly GC managed, and all value types are copied by default.
Destack keeps those defaults for plain `T`, and adds opt in ownership for performance critical paths.
In practice, this gives you single owner values and explicit borrows without changing normal TS style code.

### Ownership Modifiers

In addition to the default `T`, there are four other ownership options:
```ds
T            // type default (value or managed reference)
&readonly T  // shared borrow (read only reference)
&T           // exclusive borrow (mutable reference)
^T           // single owner handle (move-only)
^readonly T  // single owner handle (move-only, readonly)
```

Raw pointers are separate from ownership modifiers:
```ds
*readonly T  // raw pointer (readonly, unsafe)
*T           // raw pointer (mutable, unsafe)
```

Passing `^T` transfers ownership, so the previous binding becomes invalid.
Owned values are cleaned up at their last proven use, not only at lexical scope end.
Borrowed references must stay valid for their full lifetime, and strict mode rejects borrows held across suspension points like `await` and `yield`.

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

### Export Inference in Cycles

Export inference is the cycle breaker surface for cross-module type flow.
When modules form an export dependency cycle, Analyze solves exports at the SCC boundary using declared and inferred constraints from the cycle itself.
Annotations are seeds for that solve, but there is no fixed anchor count rule.
A cycle is accepted when every exported binding in the SCC is solved to a concrete type.
A cycle is rejected when any exported binding remains unsolved after surface convergence, and those exports must be annotated explicitly.

## Compatibility

**Destack aims for 100% compatibility with modern TypeScript.**

For `.ts`, `.tsx`, `.js`, and `.jsx` files, Destack parses with full compatibility—your existing code works unchanged.
For `.ds` files, a few obscure syntax patterns work differently due to built-in TSX support and additional typing features:

| Pattern | `.ts` | `.tsx` | `.ds` |
|---------|-------|--------|-------|
| `<T>() => ...` | Generic arrow | Ambiguous (use `<T,>`) | Ambiguous (use `<T,>`) |
| `(a, b, c)` | Comma operator | Comma operator | Tuple literal |

These patterns rarely appear in production code:
- The **generic arrow** ambiguity already exists in `.tsx` files—`.ds` inherits this since it supports TSX syntax natively. The workaround (`<T,>`) is standard practice in TSX codebases.
- The **comma operator** is mostly seen in minified code or obscure one-liners. Destack uses `()` for tuples instead, which is more explicit and composes better with the type system than TypeScript's `[T, U]` array syntax.

Just as `.tsx` extends `.ts` with JSX syntax (introducing the generic arrow ambiguity), `.ds` extends `.tsx` with Destack features like tuples.
Index signatures follow TypeScript numeric key coercion rules, including numeric string literals counting as number keys.

### What We Don't Support

- **Flow**: We support TypeScript only.
- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript. Non-strict ("sloppy mode") behaviors like duplicate function declarations or `yield` as an identifier are not supported. This aligns with how TypeScript modules work (always strict) and modern best practices.
- **Declaration expressions (native targets)**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with ahead-of-time compilation. Use named declarations instead. On JS targets, enable `noDynamicShapes` for portability.
- **XML namespace resolution (v1)**: Destack does not implement XML `xmlns` namespace binding semantics.
  Namespaced tree tags like `<svg:path />` are treated as intrinsic string tag names (`"svg:path"`) and routed through the active `TreeTagBuilder`.
