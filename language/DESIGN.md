# Destack Language Design

Destack is "TypeScript++" for building correct, optimal, integrated full-stack systems.
This document describes the motivation and tradeoffs in choosing TypeScript and why what was added.
Destack adds features to TypeScript that wouldn't fit in TypeScript itself (like `.tsx` or `.svelte` do).

> ---
> - Valid JavaScript is valid Destack*.
> 
> - Valid TypeScript is valid Destack*.
> 
> - Valid TSX/JSX is valid Destack*.
>
> - Destack transpiles to idiomatic TypeScript.
> ---
>
<sub>*[A few obscure syntax patterns](#compatibility) work differently in `.ds` files due to TSX-interoperability and additional typing features.</sub>

Yes, other languages with some similar features also have tried this before. 
Some are even moderately successful.
But they all fall short in interoperability, usefulness and - ultimately - adoption.
We feel that now is the time to try this again, and have made some different tradeoffs to enable adoption.

## "TypeScript++"

We're very early in software.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
But that requires unifying all the disparate pieces: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to a unified software foundation today.
JavaScript runs everywhere, everyone knows it, and it has a massive ecosystem and install base (read: every browser everywhere).
Unlike Python, the TypeScript ecosystem also has a good answer to rich frontends *and* strict modern TypeScript is a much more optimizable language. 

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it *is* TypeScript(++).
Unlike with C++, our "C" - both JavaScript and TypeScript -- still work perfectly with Destack (on JS targets), and all `++` features are opt-in and complementary.

All parts of Destack are designed to be incrementally adoptable and complementary.
This mindest also extends to the features Destack extends TypeScript with:

| Feature | Description | Tests |
|---------|-------------|-------|
| [Expressions](#expressions) | Expression extensions: "as values", ranges, patterns, `loop`, `using` | [expressions/](test/fixtures/mdtest/expressions/) |
| [Trees](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data | |
| [Annotations](#annotations) | Annotations: decorators and tags (`@`) for _any_ expression | |
| [Errors](#errors) | `Result`-first error handling with `?` propagation, no exceptions | |
| [Types](#types) | Type system extensions: newtypes, primitives, structs, tuples, constraints | [types/](test/fixtures/mdtest/types/) |
| [Comptime](#comptime) | Compile-time evaluation: precomputation, conditional compilation | |
| [Reflection](#reflection) | Types as values, runtime type descriptors, refinements, schema validation | [reflection/](test/fixtures/mdtest/reflection/) |
| [Dispatch](#dispatch) | Type-dependent dispatch: `extension`s and operator overloading | [dispatch/](test/fixtures/mdtest/dispatch/) |
| [Ownership](#ownership) | Value ownership / borrowing (`&T`, `^T`) and explicit mutability (`const`/`var`) | [ownership/](test/fixtures/mdtest/ownership/) |

## Expressions

In TypeScript, `if` is a statement, and you need a ternary or temporary to get a value out. 
Same with `switch` and most other control flow (except ternary ifs).
In Destack, everything is an expression.
The last non-statement expression (no trailing `;`) becomes the value of the expression.
This enables more ergonomic expressions for complex control flow.

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

### Patterns

Modern `match` with full pattern matching and exhaustiveness checking:

```
match (result) {
    Ok(value) => process(value)
    Err(e) if (e.retryable) => retry()
    Err(e) => fail(e)
}
```

### Loops

Infinite loops with `loop`.

```
loop {
    const input = readInput();
    if (input == "quit") { break }
    process(input);
}
```

### Using

`using` is explicit resource management, mirroring JS/TS semantics.
Resources are disposed at lexical scope exit in LIFO order, and `await using` calls the async disposer.

```
using file = openFile(path);
await using conn = openConnection();
```

## Trees

TSX syntax generalized for any tree-shaped data:

```
<Prompt>
    <System>You are helpful.</System>
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

```
function readConfig(path: string): Result<Config, IOError> {
    const text = readFile(path)?    // propagate errors with ?
    const json = parseJson(text)?
    Result.ok(Config.from(json))
}
```

The `?` operator propagates errors ergonomically, similar to Rust.
When applied to a `Result`, it returns early with the error if present.
The `??` operator provides a default value instead of propagating:

```
const config = loadConfig() ?? defaultConfig;  // use default on error
```

Both operators work via the `Try` interface, which `Result` implements.
For nullable types (`T | null`), `??` behaves exactly like TypeScript's nullish coalescing.

### Panic (throw)

`throw` is for **unrecoverable errors**: assertion failures, invariant violations, bugs.
Unlike exceptions in Java or Python, panics are not meant to be caught and recovered from.

```
function assertPositive(n: int) {
    if (n <= 0) {
        throw new Error("invariant violated: expected positive")
    }
}
```

**Native targets:** `throw` aborts the process. No stack unwinding, no catching.
This enables zero-cost error handling for the common (non-error) path.

**JS targets:** `throw` behaves as normal JavaScript throw for compatibility.

### try/catch on Result

The `try`/`catch` syntax works with `Result` types as pattern matching sugar:

```
try {
    const config = readConfig("config.json")?
    process(config)
} catch (e: IOError) {
    log("Failed to read config:", e)
}
```

This desugars to a `match` on the `Result`, with no stack unwinding involved.
It's purely syntactic convenience for handling `Result` errors.

### Design Rationale

This design is intentional:

1. **Explicit error paths**: Functions that can fail return `Result`. The type system tracks errors.
2. **Zero-cost happy path**: No exception tables, no stack unwinding machinery for native code.
3. **Bugs are bugs**: Panics indicate programmer error, not recoverable conditions.
4. **Target-appropriate semantics**: JS keeps its exception model for compatibility.

The standard library uses `Result` throughout.
If you need traditional exception semantics, target JS/TS output.

<sub>See [test/fixtures/mdtest/errors/](test/fixtures/mdtest/errors/) for specification tests.</sub>


## Types

Destack extends TypeScript's type system with precise primitives, nominal types, and readable constraints.

### Primitives

Precise numeric types beyond TypeScript's `number`:

```
const id: uint64 = 12345;
const balance: float32 = 100.50;
```

### Newtypes

Nominal wrappers that prevent mixing semantically different values:

```
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

const id = UserId(42);            // wraps scalar
const p = Point(1.0, 2.0);        // wraps tuple
const c = Config({ debug: true }); // wraps object
```

### Structs

Structs are data-oriented object types with fixed layout.
Structs are simpler than classes: no reference identity, no inheritance, just data with a name.
Structs may embed other structs to compose types, and structs can implement interfaces.

```
struct Point { x: float32, y: float32 }
```

#### Structs vs Classes

| | struct | class |
|---|---|---|
| Reference identity | ❌ No (`===` is error) | ✅ Yes (`===` compares pointers) |
| Inheritance | ❌ No (use embedding) | ✅ Yes (`extends`) |
| Default passing | Reference | Reference |
| JS output | Plain object | ES6 class |

Both structs and classes are reference types by default; ownership modifiers (`^T`, `&T`) are orthogonal.
Two structs with the same field values are equal (`==`)—structs *are* their data.
Two class instances with the same field values are not equal unless they're the same instance—classes *have* identity.

```
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
p1 == p2  // true: same data

const e1 = new Entity(1);
const e2 = new Entity(1);
e1 == e2  // false: different instances
```

#### Structs Are Nominal

Structs are nominal (like newtypes), so they must be explicitly constructed:

```
let x: Point = Point { x, y }  // ok
let x: Point = { x, y }        // error: plain object is not Point
```

For composition, structs use embedding instead of inheritance:

```
struct Transform { position: Vec3, rotation: Quat }
struct Player { ...Transform, health: int }  // embeds Transform's fields
```

#### Associated Types

Structs, classes, and interfaces can declare associated type aliases:

```
struct Cache<K, V> {
    type Entry = CacheEntry<K, V>;  // associated type
    entries: Entry[],
}
```

Associated types are resolved at compile time and can reference static parameters.
See [Associated Types](SPECIFICATION.md#associated-types) for full details.

### Constraints

`where` clauses for readable generic constraints:

```
function merge<T: int, U>(): T where (
    U: Comparable<T>
) { }
```

### The `this` Type

Destack supports TypeScript's polymorphic `this` type for instance members, and extends it to static type positions.
`this` is type-only and resolves to the surrounding receiver or containing type.

## Comptime

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
Unlike Zig or Rust macros, however, Destack's comptime fills in well-defined **typed slots** rather than enabling fully arbitrary code generation.
In practice, this `comptime` behavior and specialisation together with decorators enable most macro-style use cases without the unpredictability and compiler complexity of a "full" macro system.

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
Note here that the `LOOKUP_TABLE` must specify a type upfront.

```
function factorial(n: int): int {
    if (n <= 1) { 1 } else { n * factorial(n - 1) }
}

const FACT_10 = comptime factorial(10);    // compile time
const dynamicValue = factorial(getUserInput()); // runtime (in this case, at module initialization time)
```

Functions are not marked as "comptime" or "runtime" functions, instead, the call site determines when a function runs.
Static parameters are always "comptime" parameters, while dynamic parameters _may_ be marked `comptime` to require compile-time-known arguments.

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
struct Buffer<size: uint> {
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
  Machine interpreter during the Execute phase. Results are written back into the program
  as constants and dead branches are eliminated.

Static execution must not depend on full comptime execution. 
This avoids dependency cycles between static expressions and comptime execution.
Full comptime evaluation happens after monomorphization and lowering, with full type information available.

## Reflection

In TypeScript, types are - by design - erased at runtime.
This was critical for early adoption, but it also means you can't easily perform runtime type checks or any meaningful reflection (without additional libraries or build steps).
Destack supports `Type` as a first-class values to enable reflection with one well-defined system.

### Types as Values

Every type `T` in Destack has a corresponding runtime value of type `Type<T>`:

```
struct User { name: string, age: uint }

// User in type position: the type
let u: User = User { name: "Alice", age: 30 };

// User in value position: the type descriptor
const UserType = User;              // UserType: Type<User>
UserType.name                       // "User"
UserType.fields                     // [{ name: "name", type: string }, ...]
```

Types being values enables patterns that require runtime type information:

```
// Generic factory that knows its type parameter
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

Decorator information is accessible at runtime:

```
@deprecated("use newAPI")
function oldAPI() { }

oldAPI.decorators       // [{ name: "deprecated", arguments: ["use newAPI"] }]
```

### Refinements

Refinements add constraints to types that are checked both at compile time (when provable) and at runtime (via validation).
Refinement methods are defined via extensions on types:

```
type User = {
    name: string.minLength(1).maxLength(100),
    age: uint.max(150),
    email: string.describe("Contact email"),
}
```

The compiler checks refinements when values are provable:

```
{ name: "", age: 200, ... } satisfies User      // compile error: "" too short, 200 > max
{ name: "Alice", age: 30, ... } satisfies User  // ok
```

### Standard Library Schema

The language provides the reflection primitives; the standard library `@destack-sh/schema` provides validation utilities:

1. **Built-in (no imports)**: `Type<T>`, `.name`, `.fields`, `.is()`, decorator access
2. **Standard library**: `parse()`, `safeParse()`, refinements `.min()`, `.max()`

```
import { parse } from "@destack-sh/schema";

const data = await fetchUser();
const user = parse(User, data);    // runtime validation with errors
const user = User.parse(data);     // shorthand (schema extends Type<T>)
```

This is similar to how schema libraries work, but without the schema/type duplication:

```typescript
// Typescript: define schema, derive type
const UserSchema = t.object({ name: z.string().min(1) });
type User = t.infer<typeof UserSchema>;

// Destack: define type, validation is automatic
type User = { name: string.minLength(1) }
parse(User, data);  // User IS the schema
```

<sub>See [test/fixtures/mdtest/reflection/](test/fixtures/mdtest/reflection/) for specification tests.</sub>

### Runtime Type Identity

Runtime type identity (RTTI) is demand-driven. The compiler only emits RTTI for types that
are used at runtime (e.g., `typeOf`, `instanceof`, `any`/`unknown`, or runtime reflection).
Classes always carry a vtable pointer for dynamic dispatch and RTTI. Structs are pure data
unless RTTI is required by usage. On JS targets, RTTI-enabled values use a hidden symbol
property rather than a global WeakMap, preserving "plain object" semantics.
Native type tags are pointers to `TypeDescriptor` values rather than integer ids.
Classes reach RTTI via vtable slot 0, while thin pointers without tags recover RTTI via GC metadata.

## Dispatch

TypeScript has parametric polymorphism ("generics") but does not support type-based dispatch (by design).
Destack adds type extensions and real overloading for type-based dispatch and operator overloading.

### Extensions

Destack introduces extensions to add methods and static constants for any nominal type:

```
extension for Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions require **nominal types**—types with identity. This includes `struct`, `class`, `enum`, `newtype`, and primitive types declared in the prelude (`int32`, `string`, etc.).
Type aliases (`type X = ...`) and inline structural types (`{ x: number }`) cannot be extended.

To extend a structural shape, wrap it in a nominal type:

```
// ❌ Can't extend a type alias or inline shape
type Point = { x: number, y: number };
extension for Point { ... }  // error

// ✅ Use newtype or struct instead
newtype Point = { x: number, y: number };
extension for Point { ... }  // ok
```

Unlike Rust's blanket impls, Destack extensions only target concrete types—no `extension<T> T where T: Foo` patterns.
This is an intentional simplification: most extensions are "add methods to this specific type," and the simpler model keeps the mental overhead low.

Extensions let you add methods to any nominal type: classes, structs, enums, newtypes, even primitives and foreign types without modifying the original definition.

Extension visibility follows clear rules:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension for int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils for Date { ... }`).

### Nominal Interfaces

TypeScript interfaces are structural, i.e., any type with matching shape satisfies the interface.
Destack adds **nominal interfaces** using the `newtype` modifier on `interface` declarations:

```
// Structural interface (standard TypeScript behavior)
interface Drawable {
    draw(): void;
}
const x: Drawable = { draw() {} };  // OK: structural match

// Nominal interface (requires explicit `implements`)
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

```
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

extension for Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

For operators, Destack uses **receiver-based dispatch**: `a + b` becomes `a.add(b)`.
Relatedly, to avoid ambiguity, Destack uses **declaration order**: the first matching overload wins.

<sub>See [test/fixtures/mdtest/dispatch/](test/fixtures/mdtest/dispatch/) for specification tests.</sub>

## Ownership

TypeScript doesn't distinguish references from values in its type system, everything is implicitly GC-managed or copied purely based on type.
Destack adds opt-in explicit control, enabling a spectrum from TypeScript simplicity to Rust-level control.

### Ownership Modifiers

In addition to the default `T`, there are four other ownership options:
```
T            // automatic (TypeScript behavior, implicitly GC-managed)
&T           // borrow (read-only reference)
&mut T       // borrow (mutable reference)
^T           // ownership transfer (caller gives up ownership)
^mut T       // ownership transfer (explicitly mutable)
```

### Ownership Semantics

When you transfer ownership with `^T`, reusing the original value is an error:

```
const node = AstNode { ... }
consume(^node)    // ownership transferred
print(node.value) // ERROR: use after ownership transfer
```

`^T` values are dropped at their last proven use (non-lexical), not just at end of scope:

```
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    log("done")                       // data can be dropped before this line
}
```

This enables RAII patterns (files close, locks release, resources clean up).
Types can implement `Drop` to customize cleanup behavior.
The builtin `Error` interface is the conventional error shape for `Result<T, E>`, but any type can be used as `E`.

Strings follow the same ownership spectrum: `string` is GC-managed by default,
`&string` is a borrowed view, and `^string` is explicitly owned.

### Returning References

Functions can return borrowed references (`&T`). The compiler warns on obvious mistakes like returning a reference to a local variable, but does not enforce full lifetime tracking—GC ensures memory safety regardless.

```
function get(container: &Container): &Item {
    &container.item  // ok: borrowing from input
}

function bad(): &Point {
    const p = Point { x: 1, y: 2 }
    &p  // WARNING: returning reference to local variable
}
```

### Borrow Modes

By default, `&T` and `&mut T` are hints with warnings only.
Strict mode enforces exclusive `&mut` borrows and no-escape rules.
Strict mode enables stronger optimizations like `noalias` on `&mut`.
Enable strict mode with `borrowMode: "strict"` in `dsconfig.json`.

### Project-Level Control

Configure strictness in `dsconfig.json`:

```json
{
  "compilerOptions": {
    "noImplicitManagedType": true,   // require ^T or &T on types
    "noImplicitManagedValue": true,  // require ^x or &x on values
    "noManaged": true,               // forbid GC entirely
    "borrowMode": "strict",          // enforce &mut exclusivity rules
    "noRuntime": true                // forbid runtime features
  }
}
```

### Function-Level Control

Use decorators for per-function restrictions:

```
@noManaged
function processFrame(entities: &Entity[]) {
    // compiler error if any GC allocation happens here
}
```

<sub>See [test/fixtures/mdtest/ownership/](test/fixtures/mdtest/ownership/) for specification tests.</sub>

## Performance Strategy

Destack targets Go-level performance by default and Rust-level performance in explicit ownership modes.
The compiler uses proven optimizations and a small set of explicit controls.

Key levers:
- Escape analysis and stack promotion
- Copy elision and move elimination
- Bounds check elimination
- Devirtualization and inlining
- Monomorphization and specialization control
- Strict `&mut` borrows for `noalias`
- Explicit SIMD with scalar fallback
- LTO and PGO for whole-program optimization

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

### "TypeScript++"

Destack treats TypeScript syntax as a first-class citizen and aims for full `.ts`/`.d.ts` coverage.
Advanced TS type constructs (conditional types, mapped types, template literal types, import types, etc.) are intended to round-trip and remain visible to tooling:
- This includes tuple element modifiers, call/construct signatures, index signatures, and `this` parameters
- Type-level constructs should remain available for reflection, documentation, and compile-time evaluation
- Cross-target builds should reuse the same front-end IR and only diverge when profile-specific resolution or codegen requires it
- The `this` parameter is type-only and does not count toward call arity, but still carries normal parameter modifiers (mutability, ownership, etc.)

Comptime bridges TS types and Destack semantics:
- `comptime` can evaluate expressions that depend on `import.meta` profile data
- Type relations like `T extends U` can be used as compile-time predicates, enabling `comptime if` style gating
- The `type` operator provides an explicit way to treat types as values in comptime contexts when disambiguation is needed

### What We Don't Support

- **Flow**: We support TypeScript only.
- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript. Non-strict ("sloppy mode") behaviors like duplicate function declarations or `yield` as an identifier are not supported. This aligns with how TypeScript modules work (always strict) and modern best practices.
- **Declaration expressions (native targets)**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with ahead-of-time compilation. Use named declarations instead. On JS targets, enable `noDynamicShapes` for portability.
