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
JavaScript runs everywhere, everyone knows it, and it has a massive ecosystem.
Unlike Python, which is the #2 language, the TypeScript ecosystem has a solid answer to 1) building frontends, 2) deploying everywhere and 3) running fast(ish). 

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it *is* TypeScript++.
Unlike in `C++`, our "C" - both JavaScript and TypeScript still work perfectly in Destack, and all `++` features are opt-in and complementary.

Each feature below is independently useful, composes well with others, and can be adopted incrementally.
You can use just the features you need, and they all transpile to clean, idiomatic TypeScript.
Technically, you can even use none at all, and then Destack is just TypeScript.

| Feature | Description |
|---------|-------------|
| [Types](#types) | Type annotations are values (at runtime) |
| [Expressions](#expressions) | `if`, `match`, blocks are values |
| [Primitives](#primitives) | `int`, `int32`, `float` to refine just `number` |
| [Ranges](#ranges) | Range literals like `0..10` or `n..=m` |
| [Tuples](#tuples) | Value-oriented tuple types with `(T, T)` |
| [Trees](#trees) | TSX-like syntax for any tree-shaped data |
| [Newtypes](#newtypes) | Nominal (distinct) types |
| [Structs](#structs) | Value-oriented data types |
| [Ownership](#ownership) | Explicit `&T`, `^T` and `&var T` |
| [Constraints](#constraints) | Type guards with `where` clauses |
| [Extensions](#extensions) | Extend types and organize implementations |
| [Overloading](#overloading) | Function and operator overloading |
| [Patterns](#patterns) | Pattern matching with `match` expressions |
| [Effects](#effects) | Context declaration with `with` clauses |
| [Defer](#defer) | Cleanup with `defer` statements |

## Types

In TypeScript, types are erased at runtime and cannot affect runtime behavior.
Type erasure was instrumental to TypeScript's adoption, but it restricts what the language can be:
no runtime type information, no operator overloading, no function overloading based on types.

In Destack, type annotations are values you can inspect and use at runtime.
This enables runtime validation, automatic serialization, generic factories that know their type parameters, and reflection without separate metadata systems.
TypeScript can't do this without breaking its "types don't affect emit" principle, which is core to its design.

## Expressions

In TypeScript, `if` is a statement—you need a ternary or temporary to get a value.
In Destack, everything is an expression:

```
const result = if condition { computeA() } else { computeB() };
const label = match state { 
    State.Ready => "go", 
    State.Loading => "wait" 
};

function add(a: int, b: int): int {
    a + b // -> return a + b;
}
```

The last non-statement expression (no trailing `;`) becomes the value of the expression.
Expressions as values reduce temporary variables and can make data flow more explicit.
Of course, this is optional, and you can still use `return` and all other control flow just as before.

## Primitives

TypeScript's `number` is always a 64-bit float.
Destack adds precise types (`int32`, `uint64`, `float32`) as opt-in alternatives, plus raw strings and byte literals.

```
const id: uint64 = 12345;
const data: uint8[] = b"binary";
const raw = r#"no \n escaping"#;
```

Precise types improve correctness for IDs, indices, and binary protocols, and enable eventual native compilation.
Use `number` when precision doesn't matter, precise types when it does.
This is like Rust's numeric types. TypeScript can't add new primitive types without JavaScript support.

## Ranges

Destack adds range literals for iteration and slicing:

```
for i in 0..10 { }      // exclusive
for i in 0..=10 { }     // inclusive
```

Ranges make loops cleaner and more explicit.

## Tuples

Destack adds explicit tuple syntax and destructuring with parentheses:

```
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

Explicit tuple syntax improves clarity when working with fixed-size heterogeneous data.
TypeScript has tuples via array syntax `[number, number]`, but the syntax conflates tuples with arrays.

## Trees

TSX brought declarative tree syntax to React, and it has been massively successful.
Destack generalizes it so `<element />`-style syntax works with any tree-shaped problem:

```
<Prompt>
    <System>You are helpful.</System>
    <User>{message}</User>
</Prompt>
```

Entity trees, prompt structures, scene graphs, configuration—all with the same syntax, within the same file.

## Newtypes

TypeScript's type aliases are structural — two aliases for `string` are interchangeable.
There are many ways to "brand" types in TypeScript, but they're all a little clumsy.
Destack adds `newtype` for nominal (distinct) types:

```
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64
```

Newtypes prevent entire categories of bugs by making semantically different values incompatible at compile time.
As a bonus, because types are first-class citizens in Destack, we get to associate methods and constants with newtypes (or any other types) using `extension` (see below).

## Structs

Destack adds `struct` for data-oriented types with value semantics:

```
struct Point { x: float32, y: float32 }
```

Structs are simpler and more predictable than classes for plain data.
Unlike classes, structs are passed by value (copied) by default and have no constructor ceremony.
This is like Rust structs or C structs.
TypeScript only has classes and interfaces, both reference-based.

## Ownership

TypeScript doesn't distinguish references from values—everything is implicitly reference-counted or copied based on type.
Destack adds opt-in explicit control for certain situations:

```
T            // automatic value or reference (just like before)
&T           // immutable reference
&var T       // mutable reference
^T           // value ("copy semantics")
^var T       // mutable value ("move semantics")
```

Explicit ownership enables manual memory management patterns and clearer reasoning about mutation and aliasing.
This is like Rust's references, though without the full borrow checker (for now).
TypeScript can't express reference vs. value semantics in its type system.

## Constraints

TypeScript's generic constraints use inline `extends` syntax, which can get a bit clumsy.
Destack treats static parameters as "real" parameters instead of just polymorphic typing, 
 and Destack additionally supports `where` clauses for refining constraints:

```
function merge<T: int, U>(): T where (
    U: Comparable<T>
) { }
```

Where clauses keep function signatures readable when constraints are complex.
This is like Rust's `where` clauses or Swift's generic constraints.

## Extensions

TypeScript extends types via prototype mutation or declaration merging, both with footguns.
Destack adds scoped, type-safe extensions:

```
extension Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions let you add methods to any type—structs, enums, even primitives and foreign types—without modifying the original and without global side effects.
This is like Rust's `impl` blocks or Swift's extensions.
TypeScript's declaration merging is global and can cause conflicts.

## Overloading

TypeScript has limited function overloading via type-only declarations that all share one implementation, and no operator overloading.
Destack supports real function and operator overloading with distinct implementations:

```
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

extension Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

Real overloading reduces boilerplate and enables natural mathematical notation.
The compiler picks the right implementation based on argument types and Destack errors obviously if resolution is not possible or sensible.

For operators, Destack uses **receiver-based dispatch**: `a + b` desugars to `a.add(b)`, so the left operand's type determines which implementation is called.
This matches TypeScript's method dispatch semantics and keeps overload resolution simple and predictable.

For function overloads, Destack uses **declaration order**: the first matching overload wins.
This matches TypeScript's overload resolution and means more specific overloads should be declared before general ones.
The compiler warns if an overload is shadowed by an earlier declaration that always matches first.

## Patterns

TypeScript's `switch` comes from C tradition and is limited to value equality with no destructuring.
Destack adds modern `match` with full pattern matching:

```
match result {
    Ok(value) => process(value)
    Err(e) if e.retryable => retry()
    Err(e) => fail(e)
}
```

Pattern matching eliminates entire classes of bugs through exhaustiveness checking and makes complex conditionals more readable.
Destack's `match` compiles to a clean switch and/or if-else construct in TypeScript.

## Effects

TypeScript functions don't declare their side effects—any function might do I/O, allocate, or throw.
Destack adds optional `with` clauses for effect tracking:

```
function readFile(path: string): string with FileSystem { }
function pure<T>(x: T): T with !Allocation { }
```

Effect tracking makes function capabilities explicit and enables the compiler to enforce purity constraints.
Declare what a function can do, or explicitly forbid effects.
This is like algebraic effects in research languages, but pragmatic and opt-in.
TypeScript has no way to express or enforce effect constraints.

## Defer

Destack adds optional `defer` for cleanup that runs when scope exits:

```
const file = open(path);
defer file.close();
// file.close() runs when this scope exits, however it exits
```

Defer ensures cleanup happens regardless of how a function exits, without try/finally boilerplate.
This is like Go's `defer` or Swift's `defer`.
TypeScript requires try/finally or explicit cleanup, which is verbose and easy to forget.
