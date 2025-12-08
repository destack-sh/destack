# Destack Language Design

Destack extends TypeScript for building correct, optimal, integrated full-stack systems.
You can think of Destack like "TypeScript++", except we're still fully interoperable both ways.
This document describes the motivation and tradeoffs in choosing TypeScript and why what was added.
Destack adds features to TypeScript that wouldn't fit in TypeScript itself (like `.tsx` or `.svelte` do).

> ---
> - **Valid JavaScript is valid Destack.**
> - **Valid TypeScript is valid Destack.**
> - **Valid TSX/JSX is valid Destack.**
> - **Destack transpiles to idiomatic TypeScript.**
> ---

Yes, other languages with some similar features also have tried this before. 
Some are even moderately successful.
But they all fall short in interoperability, usefulness and - ultimately - adoption.
We feel that now is the time to try this again, and have made some different tradeoffs to enable adoption.

## TypeScript++

We're very early in software.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
But that requires unifying all the disparate pieces: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to a unified software foundation today.
JavaScript runs everywhere, everyone knows it, and it has a massive ecosystem and install base.
Unlike Python, the TypeScript ecosystem also has a good answer to rich frontends and is a much more optimizable language (especially in strict TypeScript). 

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it *is* TypeScript(++).
Unlike with C++, our "C" - both JavaScript and TypeScript -- still work perfectly in Destack, and all `++` features are opt-in and complementary.

Each feature below is independently useful, composes well with others, and can be adopted incrementally.
You can use just the features you need, and they all transpile to clean, idiomatic TypeScript.
Technically, you can even use none at all, and then Destack is just TypeScript.

| Feature | Description |
|---------|-------------|
| [Expressions](#expressions) | Expression extensions: ranges, tuples, patterns, `loop`, `using` |
| [Trees](#trees) | Tree literals: TSX-like syntax generalized for any tree-shaped data |
| [Annotations](#annotations) | Annotations: decorators (`@`) for any expression |
| [Types](#types) | Type system extensions: newtypes, primitives, structs, constraints |
| [Reflection](#reflection) | Types as values, runtime type descriptors, refinements, schema validation |
| [Dispatch](#dispatch) | Type-based dispatch: extensions and overloading |
| [Ownership](#ownership) | Value ownership (`&T`, `^T`), mutability (`const`/`var`), and explicit dispatch |

## Expressions

In TypeScript, `if` is a statement—you need a ternary or temporary to get a value.
In Destack, everything is an expression.
The last non-statement expression (no trailing `;`) becomes the value.

```
const result = if (condition) { computeA() } else { computeB() };

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

Explicit tuple syntax with parentheses (clearer than TypeScript's `[T, U]` array syntax):

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

## Trees

TSX-like syntax generalized for any tree-shaped data:

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

Destack extends decorators (`@`) to work on any declaration, statement, or expression—not just class members.

```
@deprecated("use newAPI instead")
function oldAPI() { }

@memoize
function expensive() { }

@unroll
for (let i = 0; i < 4; i++) { }
```

Decorator behavior depends on what it resolves to:
- **Function**: Transforms the target (standard decorator semantics)
- **Newtype**: Compile-time metadata, stripped in output (for hints like `@unroll`, `@inline`)

TypeScript decorators copy-pasted into Destack work as expected.

## Types

Destack extends TypeScript's type system with precise primitives, nominal types, and readable constraints.

### Primitives

Precise numeric types beyond TypeScript's `number`:

```
const id: uint64 = 12345;
const balance: float32 = 100.50;
```

### Newtypes

Nominal (distinct) types that prevent mixing semantically different values:

```
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

const id = UserId(42);           // scalar newtype
const p = Point(1.0, 2.0);       // tuple newtype
const c = Config { debug: true }; // struct newtype
```

### Structs

Structs are data-oriented object types with fixed layout and value semantics.
Basically, structs are simpler classes for plain data objects with stricter guarantees and control.

```
struct Point { x: float32, y: float32 }
```

Structs are nominal (like newtypes), so they must be created or coerced explicity:

```
let x: Point = new Point(x, y) // ok
let x: Point = Point { x, y }  // ok
let x: Point = { x, y }        // error
```

### Constraints

`where` clauses for readable generic constraints:

```
function merge<T: int, U>(): T where (
    U: Comparable<T>
) { }
```

## Reflection

In TypeScript, types are - by design - erased at runtime.
This was critical for early adoption, but it also means you can't easily perform runtime type checks or any meaningful reflection (without additional libraries or build steps).
Destack makes types first-class runtime values in one integrated system, enabling reflection with one well-defined and well-documented system.

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

## Dispatch

TypeScript has parametric polymorphism ("generics") but does not support type-based dispatch (by design).
Destack adds type extensions and real overloading for type-based dispatch and operator overloading.

### Extensions

Destack introdues extensions to add methods and static constants for any type:

```
extension Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions let you add methods to any type: classes, structs, enums, even primitives and foreign types without modifying the original definition and without (hidden) global side effects.

### Overloading

Real function and operator overloading with distinct implementations:

```
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

extension Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

For operators, Destack uses **receiver-based dispatch**: `a + b` desugars to `a.add(b)`.
For function overloads, Destack uses **declaration order**: the first matching overload wins.

## Ownership

TypeScript doesn't distinguish references from values—everything is implicitly reference-counted _or_ copied purely based on type.
Destack adds opt-in explicit control over three orthogonal aspects of ownership:

### Value Ownership

Control whether data is passed by reference or by value:

```
T            // automatic (TypeScript behavior)
&T           // reference (shared access)
^T           // value (copy semantics)
```

### Mutability Ownership

Control whether bindings can be mutated:

```
&const T     // immutable reference
&var T       // mutable reference
^const T     // immutable value
^var T       // mutable value
```

### Dispatch Behavior

Control how polymorphic calls are dispatched:

```
&T             // automatic dispatch
&implements T  // explicit dynamic interface dispatch
&extends T     // explicit dynamic class dispatch
```

Explicit dispatch enables devirtualization and other optimizations when the compiler can prove static dispatch is safe.
It's closer to Mojo's approach: explicit control when you need it, automatic behavior when you don't.
