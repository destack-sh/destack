# Destack Language Design

Destack is "TypeScript++" for building correct, optimal, integrated software systems.
This document describes the motivation and tradeoffs in choosing TypeScript and why we added what.
(Basically, Destack adds features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do).

Of course, other languages with some similar features also have tried this before, and some are moderately successful.
But they all fall short in interoperability, usefulness and - ultimately - adoption.
We feel that now is the time to try this again, and have made some different tradeoffs to enable TypeScript to cover many more usage scenarios.

## "TypeScript++"

We're very early in software.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
But that requires unifying all the disparate pieces: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to a unified software foundation today.
JavaScript runs everywhere, everyone knows it, and it has a massive ecosystem and install base (i.e., every browser everywhere).
Unlike Python, the TypeScript ecosystem also has a good answer to rich frontends *and* strict modern TypeScript is a much more optimizable language (as evidenced by V8 and JSC coming within touching distance of Go and C# in some scenarios). 

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it *is* TypeScript(++).
Unlike with C++, our "C" - both JavaScript and TypeScript -- still work perfectly with Destack (on JS/TS targets), and the `++` features are opt-in and complementary.

| Feature | Description | Tests |
|---------|-------------|-------|
| [Expressions](#expressions) | Expression extensions: "as values", ranges, patterns, `loop`, `using` | [expressions/](test/fixtures/specification/expressions/) |
| [Trees](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data | |
| [Annotations](#annotations) | Annotations: decorators and tags (`@`) for _any_ expression | |
| [Errors](#errors) | `Result`-first error handling with `?` and `??` propagation, no exceptions | |
| [Types](#types) | Type system extensions: newtypes, primitives, structs, tuples, constraints | [types/](test/fixtures/specification/types/) |
| [Comptime](#comptime) | Compile-time evaluation: precomputation, conditional compilation | |
| [Reflection](#reflection) | Types as values, runtime type descriptors, refinement metadata, schema validation | [reflection/](test/fixtures/specification/reflection/) |
| [Dispatch](#dispatch) | Type-dependent dispatch: `extension`s and operator overloading | [dispatch/](test/fixtures/specification/dispatch/) |
| [Ownership](#ownership) | Value ownership / borrowing (`&T`, `^T`) and explicit mutability (`const`/`var`) | [ownership/](test/fixtures/specification/ownership/) |

## Interactive Execution

Destack treats interactive workflows as first class language use cases.
REPLs and notebooks use the same compiler pipeline and VM as production code.
Incremental compilation relies on module signatures and profile versions to keep latency low without query systems.
The compiler, daemon, and LSP share a canonical cache format, with sidecars for consumer-specific metadata.


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

If let is sugar for matching a value with a pattern in a conditional.
Bindings from the pattern are scoped to the then branch.
If let without else yields void.

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

Arrays are dense on native targets: array literals do not permit holes, and index
access is bounds checked. `a[i]` returns the element type and out of bounds access
follows the `boundsChecks` and `checkFailure` policies.

### Patterns

Modern `match` with full pattern matching and exhaustiveness checking:

```
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
`Try.branch()` returns a structural `TryBranch<T, E>` shape.
Structural `TryBranch` compatibility is based on object shapes, not nominal structs or newtypes.
`Try.fromError` is required when a `?` propagates out of the enclosing function.
The `??` operator coalesces nullish values before and after a single `Try` unwrap.
This keeps `Result<T, E> | null` ergonomic without recursive unwrapping.

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

### try/catch with Result and exceptions

The `try`/`catch` syntax handles exceptions and explicit `Try` propagation:

```
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

```
try {
    riskyOperationA()?;
} catch match e {
    NumericError(x) => Error(@format("bad number: {x}"))
    FormatError => Error(@format("bad format {e}"))
    _ => Error(@format("unknown error: {e}"))
}
```

### Design Rationale

This design is intentional:

1. **Explicit error paths**: Functions that can fail return `Result`. The type system tracks errors.
2. **Zero-cost happy path**: No exception tables, no stack unwinding machinery for native code.
3. **Bugs are bugs**: Panics indicate programmer error, not recoverable conditions.
4. **Target-appropriate semantics**: JS keeps its exception model for compatibility.

The standard library uses `Result` throughout.
If you need traditional exception semantics, target JS/TS output.

<sub>See [test/fixtures/specification/](test/fixtures/specification/) for specification tests.</sub>


## Types

Destack extends TypeScript's type system with precise primitives.
It adds nominal types and readable constraints.

### Inference

Destack requires explicit types at public boundaries.
This keeps inference local, fast, and predictable.
There is no whole program inference or Hindley-Milner style generalization.

Most non local constructs should be explicitly typed:
- Exported functions, methods, and constructors annotate dynamic parameters and return types.
- Public fields and properties declare types.
- Function types in type declarations annotate parameters and return types.

Local inference is fully supported wherever convenient and unambiguous:
- Static parameters may include types but are not required.
- Static parameters default to type parameters unless a value constraint or value default is provided.
- Lambdas may omit parameter and return types when a contextual type is available.
- Local bindings may infer types from their initializer.
- Object literal fields may omit annotations when the binding is typed or uses `satisfies`.

### Primitives

Precise numeric types beyond TypeScript's `number`:

```
const id: uint64 = 12345;
const balance: float32 = 100.50;
```

Destack keeps `number` as the JS-compatible numeric supertype (aliased to `float64`).
`int`/`uint` and `float` use the compiler's default widths (32-bit ints, 64-bit floats by default).
Pointer-sized integers are spelled `isize` and `usize`.

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

Structs are data-oriented value types with fixed layout.
Structs have no identity or inheritance, just data with a name.
Structs may embed other structs to compose types, and structs can implement interfaces.
Struct values can be boxed when a reference is required, which allocates managed storage without changing the struct type.
Boxing is compiler inserted and does not introduce a surface `Box<T>` type.
Boxing copies the value into managed storage, and repeated boxing creates distinct reference identities.
Structural object types remain reference types, even when written as type aliases.
Type aliases inherit the semantics of the underlying type.
Struct declarations require a name and cannot be anonymous.

```
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

```
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

```
let x: Point = Point { x, y };  // ok
let x: Point = { x, y };        // error: plain object is not Point
```

For composition, structs use embedding instead of inheritance:

```
struct Transform { position: Vec3; rotation: Quat; }
struct Player { ...Transform; health: int; }  // embeds Transform's fields
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

Destack supports TypeScript's polymorphic `this` type for instance members, and also allows it in static type positions.
`this` is type-only and resolves to the surrounding receiver or containing type.

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
  VM interpreter during the Execute phase. Results are written back into the program
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

Runtime type guards use `x is T` or `T.is(value)` for general types.
The `instanceof` operator is reserved for class identity checks.

### Decorator Metadata

Decorator information is accessible at runtime:

```
@deprecated("use newAPI")
function oldAPI() { }

oldAPI.decorators       // [{ name: "deprecated", arguments: ["use newAPI"] }]
```

### Refinements

Refinements attach metadata to types for schema and validation tooling.
The compiler treats refinements as opaque metadata and does not validate them.
Refinement methods are defined via extensions on types:

```
type User = {
    name: string.minLength(1).maxLength(100),
    age: uint.max(150),
    email: string.describe("Contact email"),
}
```

Refinements do not affect type checking or narrowing.
Use explicit `where` clauses or guards for static enforcement, and runtime validation when needed.

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

// Destack: define type, validation is explicit
type User = { name: string.minLength(1) }
parse(User, data);  // User IS the Type (schema)
```

<sub>See [test/fixtures/specification/reflection/](test/fixtures/specification/reflection/) for specification tests.</sub>

### Runtime Type Identity

Runtime type identity (RTTI) is demand-driven.
The compiler only emits RTTI for types that are used at runtime.
Examples include `typeOf`, `T.is`, `x is T`, `instanceof` for classes, `any`/`unknown`, and runtime reflection.
Polymorphic classes carry a vtable pointer for dynamic dispatch and RTTI.
Non-polymorphic classes may omit the vtable pointer and rely on metadata or fat pointers when RTTI is required.
Structs are pure data unless RTTI is required by usage.
On JS targets, RTTI-enabled values use a hidden symbol property rather than a global WeakMap, preserving "plain object" semantics.
Native type tags are `TypeTag` handles that point to `TypeDescriptor` values.
Classes reach RTTI via vtable slot 0 when present, while thin pointers without tags recover RTTI via GC metadata.

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

Extension static parameters bind positionally to the target type's static parameters.
The target type expression can reorder those parameters, and that order defines how receiver static arguments map to extension parameters.
Defaults on the target type apply when static arguments are omitted at the use site.

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

### Dynamic Resolution

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant.
If all variants resolve to the same symbol, the call is static.
If the symbols differ, the compiler records a dynamic resolution and reifies it into `if (receiver is Type)` branches.

Dynamic resolution only applies when every union variant exposes the member.
Arguments must satisfy all candidate signatures, and the resulting type is the union of per-candidate return types after substitutions.
Extension methods participate in member resolution, too.

### Dispatch Tables

Dispatch chooses between direct calls, class virtual dispatch, interface dispatch, and union reification.
The design goal is to keep class overhead minimal while preserving TypeScript structural semantics.
Layout and slot details live in the specification and the Lower documentation.

## Ownership

TypeScript does not encode ownership in its type system.
Reference types are implicitly GC managed, and value types are copied by default.
Destack adds opt in explicit control, enabling a spectrum from TypeScript simplicity to Rust level control.

### Ownership Modifiers

In addition to the default `T`, there are four other ownership options:
```
T            // type default (value or managed reference)
&T           // borrow (read only reference)
&mut T       // borrow (mutable reference)
^T           // ownership transfer (caller gives up ownership)
^mut T       // ownership transfer (explicitly mutable)
```

Raw pointers are separate from ownership modifiers:
```
*T           // raw pointer (unsafe)
*mut T       // raw pointer (mutable, unsafe)
```
**Borrow semantics:**
- `&T` and `&mut T` are safe borrows verified by the borrow check pass.
- Borrows are created by `field.addr`, `element.addr`, and by calls that return borrowed references with lifetimes.
- A borrow ends when the reference value is no longer live.
- Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
- Derived borrows carry provenance so dropping any origin invalidates the derived borrows.
- Dropping or freeing a value while it is borrowed is always an error.
- In strict mode, conflicting borrows and invalidating stores are errors.
- In lenient mode, the same situations produce warnings.

**Raw pointers:**
- `*T` and `*mut T` are unsafe pointers with no borrow tracking.
- Raw pointers may be null or dangling and allow pointer arithmetic.
- Converting between borrowed references and raw pointers is always explicit.
- Raw pointers do not imply ownership or drop behavior.
- `&const T` and `*const T` are accepted but redundant and format as `&T` and `*T`.

### Address Spaces

References can target explicit address spaces for native and accelerated targets.
The default is `generic`, which maps to the target's normal memory.
Non generic address spaces are only valid for borrowed and raw references.
`constant` references are always immutable.

Address spaces are spelled with `addrspace(name)` or `addrspace(7)` on a reference:

```
ref<raw addrspace(shared) i32>
ref<raw addrspace(7) mut i32>
```

Address space changes are explicit (and lower to the `addrspace.cast` intrinsic).
The VM provides deterministic host side models for non generic address spaces when available.

### Ownership Semantics

When you transfer ownership with `^T`, reusing the original value is an error:

```
const node = AstNode { ... }
consume(^node)    // ownership transferred
print(node.value) // ERROR: use after ownership transfer
```

`^T` values are dropped at their last proven use (non lexical), not just at end of scope:

```
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    log("done")                       // data can be dropped before this line
}
```

This enables RAII patterns (files close, locks release, resources clean up).

**Drop behavior:**
- `Drop` is a marker interface that opts a type into last use cleanup.
- Types that implement `Drop` must also implement `Symbol.dispose`, which is invoked by the drop glue.
- `using` always calls `Symbol.dispose`, even without `Drop`.
- Owned values are dropped at their last proven use unless `using` is specified.
- `using` bindings drop at scope end and cannot be moved.
- `^T` controls ownership transfer and move semantics.
- `using` controls drop timing and does not imply ownership.
- Combine `using` with `^T` for deterministic cleanup of owned values.

### Allocation and Drop

Ownership modifiers determine how values are allocated and cleaned up:

| Source | Allocation | Cleanup | Semantics |
|--------|------------|---------|-----------|
| `T` (plain) | `managed.alloc` | GC | GC handles everything |
| `^T` (owned) | `raw.alloc` | `raw.drop` | dispose + deallocate |
| `&T` (borrow) | none | none | points to someone else's memory |

The `raw.drop` operation performs **drop glue**: drop owned fields (reverse declaration order), call `Symbol.dispose` if the type implements `Drop`, then deallocate.
The optimizer may promote `raw.alloc` to `stack.alloc` via escape analysis when the value doesn't escape the function.
Stack allocated owned values use `stack.drop`, which runs the same drop glue but skips deallocation (the frame handles it).

For manual memory without destructors (FFI, low-level code), use `raw.free` directly instead of `raw.drop`.

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

Strings follow the same ownership spectrum: `string` is GC managed by default,
`&string` is a borrowed view, and `^string` is explicitly owned.
`Slice` is an explicit view type with a pointer and length.
Borrowing an array object does not imply a slice view.

### Ownership Conversions

Ownership conversions are explicit, except for borrows inserted at reference boundaries.
Implicit ownership conversions only create borrows and never transfer ownership.

Implicit conversions:
- `T` → `&T` or `&mut T` when a reference type is required and the value is addressable
- `^T` → `&T` to borrow from an owned value
- `&mut T` → `&T` to reborrow as shared

Explicit conversions:
- `T` ↔ `^T` require explicit ownership operators or helper calls
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

### Borrow Modes

By default, `&T` and `&mut T` are hints and violations produce warnings.
Strict mode enforces exclusive `&mut` borrows and no escape rules.
Strict mode enables stronger optimizations like `noalias` on `&mut`.
Enable strict mode with `borrowMode: "strict"` in `dsconfig.json`.
Borrow modes do not apply to raw pointers.

### Lifetime Annotations

When returning references or aggregates containing references, the compiler needs to know which input parameters the return value borrows from. This is usually inferred:

- Single `&T` parameter → return borrows from it
- `&self`/`&this` method → return borrows from receiver
- Multiple `&T` parameters → conservative (borrows from all)

When inference is too conservative, use `@lifetime` to be explicit:

```
// explicit: only borrows from 'a', not 'b'
function first(a: &string, b: &string): @lifetime("a") &string {
    return a;
}

// may borrow from either
function pick(a: &string, b: &string): @lifetime("a", "b") &string {
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

### Project-Level Control

Configure strictness in `dsconfig.json`:

```json
{
  "compilerOptions": {
    "noImplicitManaged": true,       // require ^T or &T on types and values
    "noManaged": true,               // forbid GC-managed defaults and allocations
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
    // compiler error if any managed value is used or allocated here
}
```

<sub>See [test/fixtures/specification/ownership/](test/fixtures/specification/ownership/) for specification tests.</sub>

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
- LTO and PGO for package and program scope optimization

Optimization defaults to module scope for fast builds.
Target ltoMode selects the optimization scope.
Thin LTO runs at package scope and Full LTO runs at program scope.
Auto selects Thin LTO at O4 and disables LTO at lower levels.
Compilation unit refers to the selected optimization scope when LTO is enabled.

## Module Imports

Destack supports importing various file types beyond code modules, following Bun's approach to asset imports.

### Data Modules (JSON, TOML, YAML)

Data files are parsed at compile time and typed structurally:

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

### Text Modules

Text files (markdown, CSS, HTML, plain text) import as `string`:

```
import readme from "./README.md";
// readme: string
```

### Binary Modules

Binary files (images, fonts, wasm, etc.) import as `uint8[]`:

```
import icon from "./icon.png";
// icon: uint8[]
```

### Import Attributes

Override the default loader with import attributes:

```
import data from "./config.toml" with { type: "json" };  // parse as JSON
import raw from "./data.json" with { type: "text" };     // import as string
import bytes from "./file.txt" with { type: "binary" };  // import as uint8[]
```

The same file with different loaders produces different modules.

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

### "TypeScript++"

Destack treats TypeScript syntax as a first-class citizen and aims for full `.ts`/`.d.ts` coverage.
Advanced TS type constructs (conditional types, mapped types, template literal types, import types, etc.) are intended to round-trip and remain visible to tooling:
- This includes tuple element modifiers, call/construct signatures, index signatures, and `this` parameters
- Type-level constructs should remain available for reflection, documentation, and compile-time evaluation
- Cross-target builds should reuse the same front-end IR and only diverge when profile-specific resolution or codegen requires it
- The `this` parameter is type-only and does not count toward call arity, but still carries normal parameter modifiers (mutability, ownership, etc.)

Comptime bridges TS types and Destack semantics:
- `comptime` can evaluate expressions that depend on `import.meta` profile data.
- Type relations like `T extends U` can be used as compile-time predicates.
- The `type` operator provides an explicit way to treat types as values in comptime contexts.
- `@if` gates declarations and members using static expressions like `import.meta`.

### What We Don't Support

- **Flow**: We support TypeScript only.
- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript. Non-strict ("sloppy mode") behaviors like duplicate function declarations or `yield` as an identifier are not supported. This aligns with how TypeScript modules work (always strict) and modern best practices.
- **Declaration expressions (native targets)**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with ahead-of-time compilation. Use named declarations instead. On JS targets, enable `noDynamicShapes` for portability.
