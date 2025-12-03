# Destack Language Design

Destack extends TypeScript for building correct, optimal, integrated full-stack systems.
You can think of Destack like "TypeScript++", except we're still fully interoperable both ways.
This document describes the motivation and tradeoffs in choosing TypeScript and why what was added.
Destack adds features to TypeScript that wouldn't fit in TypeScript itself (like `.tsx` or `.svelte` do) without splintering the ecosystem.

> ---
> - **Valid JavaScript is valid Destack.**
> - **Valid TypeScript is valid Destack.**
> - **Destack transpiles to idiomatic TypeScript.**
> - **JavaScript, TypeScript, and Destack coexist.**
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
| [Expressions](#expressions) | Expression extensions: ranges, tuples, patterns, trees, `loop`, `defer` |
| [Types](#types) | Type system extensions: runtime types, newtypes, primitives, structs, constraints |
| [Polymorphism](#polymorphism) | Polymorphism: extensions and overloading |
| [Annotations](#annotations) | Annotations: tags (`#`) and extended decorators (`@`) |
| [Context](#context) | Context: effect declarations with `with` clauses |
| [Ownership](#ownership) | Ownership: Value ownership (`&T`, `^T`), mutability (`const`/`var`), and dispatch behavior |

## Expressions

In TypeScript, `if` is a statement—you need a ternary or temporary to get a value.
In Destack, everything is an expression.
The last non-statement expression (no trailing `;`) becomes the value.

```
const result = if condition { computeA() } else { computeB() };

function add(a: int, b: int): int {
    a + b // implicit return
}
```

### Ranges

Range literals for iteration and slicing:

```
for i in 0..10 { }      // exclusive
for i in 0..=10 { }     // inclusive
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
match result {
    Ok(value) => process(value)
    Err(e) if e.retryable => retry()
    Err(e) => fail(e)
}
```

### Trees

TSX-like syntax generalized for any tree-shaped data:

```
<Prompt>
    <System>You are helpful.</System>
    <User>{message}</User>
</Prompt>
```

### Loop and Defer

Infinite loops with `loop`, cleanup with `defer`:

```
loop {
    const input = readInput();
    if input == "quit" { break }
    process(input);
}

const file = open(path);
defer file.close();
// file.close() runs when this scope exits
```

## Types

In TypeScript, types are erased at runtime and cannot affect runtime behavior.
In Destack, type annotations are values you can inspect and use at runtime.
This enables runtime validation, automatic serialization, generic factories that know their type parameters, and reflection without separate metadata systems.

### Primitives

Precise numeric types beyond TypeScript's `number`, plus raw strings and byte literals:

```
const id: uint64 = 12345;
const data: uint8[] = b"binary";
const raw = r#"no \n escaping"#;
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

Newtypes combined with structs enable idiomatic Result types:

```
struct Ok<T> { kind: 'ok' = 'ok', value: T }
struct Err<E> { kind: 'err' = 'err', error: E }
type Result<T, E> = Ok<T> | Err<E>
```

### Structs

Data-oriented types with fixed layout, value semantics (simpler than classes for plain data objects with more guarantees).
Conceptually, Destack structs are similar to the TC39 struct proposal with additional "systems-level" features.

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

### Refinements

Types are values and can be manipulated as expressions - at compile time and at runtime.
This enables refinements of types, like is commonly used in schema libraries, without extra ceremony:

```
type User = {
    name: string.minLength(1).maxLength(100),
    age: uint.max(150),
    email: string.describe("Contact email"),
}
```

Refinements build on types-as-values and extensions: every type exists at runtime as a descriptor, and refinement methods attach constraints to these descriptors.
The compiler checks refinements when provable:

```
{ name: "", age: 200, ... } satisfies User      // compile error: "" too short, 200 > max
{ name: "Alice", age: 30, ... } satisfies User  // ok
```

The additional refinements and runtime validation are provided opt-in via the standard library `destack-schema`.
Foreign and "unproven" data can be explicitly coerced or dynamically checked:

```
import { parse } from "destack-schema";

const data = await fetchUser();
const user = parse(User, data);    // explicit runtime validation
const user = User.parse(data);     // shorthand explicit runtime validation
```

## Polymorphism

TypeScript extends types via prototype mutation or declaration merging, both with footguns.
TypeScript has limited function overloading via type-only declarations that all share one implementation.
Destack adds proper extensions and real overloading.

### Extensions

Scoped, type-safe extensions for any type:

```
extension Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions let you add methods to any type: structs, enums, even primitives and foreign types—without modifying the original and without global side effects.

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

## Annotations

Destack adds tags (`#`) for structured metadata and extends decorators (`@`) to work on any declaration.

### Tags

Compile-time metadata attached to declarations:

```
#Performance
#deprecated("use newAPI instead")
function oldAPI() { }
```

Tags are structured and queryable, unlike comments.

### Decorators

TypeScript decorators extended to work on any declaration (functions, variables, structs), not just class members:

```
@memoize
@route("/api/users")
function getUsers() { }
```

TypeScript decorators copy-pasted into Destack work as expected.

## Context

TypeScript functions don't declare their side effects—any function might do I/O, allocate, or throw.
Destack adds optional `with` clauses for effect tracking:

```
function readFile(path: string): string with FileSystem { }
function pure<T>(x: T): T with !Allocation { }
```

Effect tracking makes function capabilities explicit and enables the compiler to enforce purity constraints.
This is like algebraic effects in research languages, but pragmatic and opt-in.

## Ownership

TypeScript doesn't distinguish references from values—everything is implicitly reference-counted or copied based on type.
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
