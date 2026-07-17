---
title: Types
description: Primitives, nominal and value types, constraints, representation, and reflection.
order: 21
---

# Types

Destack extends TypeScript's type system with precise primitives, nominal types ("`newtype`s"), value types ("`struct`s"), tuples, `where`-based constraints, and some additional niceties.

## Primitives

Destack is based on TypeScript, and TypeScript inherits its main primitive types from JavaScript: `string`, `boolean`, `number`, `bigint`, and `symbol`, plus the `null` and `undefined` sentinels.
Destack evolves this set into a serious set of primitive types:
- precise numeric types beyond `number`, with variable-width signed and unsigned integers (`int8`, `uint32`, `int17`) as well as concrete float formats (`float16`, `float32`, `float64`)
- pointer-sized integers, i.e. integers as wide as the target pointer size, spelled `isize` and `usize`
- `int` and `uint` as aliases to `int64` and `uint64`
- `number` becomes an alias for `float`, and `float` becomes an alias for `float64`
- `char` as a single Unicode scalar value, distinct from `string`

It should be noted that `string` and `bigint` are not really "special" in Destack like they are in TypeScript, they are just aliases to the standard library `String` and `BigInt` classes.
They feel the same, though.
In general, Destack follows TypeScript behavior as exactly as possible for a sound and strict type system, including the exact same widening rules where numeric literals start as exact values and can flow into any numeric type that can represent them.

```ds
const id: uint64 = 12345;
7 satisfies uint3;
7 satisfies uint2; // ERROR: 7 does not fit uint2

const exact: int = 42;
exact satisfies int64;

const balance: float = 100.50;
balance satisfies float64;

const n: number = 1.0;
n satisfies float;

const initial: char = 'A';
const input: unknown = readInput();
```

## Unknown

TypeScript has two "top" types: `unknown` and `any` can contain _all_ other types.
Of course, `any` is unsound, because everything can be assigned to and from `any` without any checks, so Destack forbids it in favor of the explicit `unknown`.
`unknown` is really just a transparent top constraint, and so it behaves like an interface with zero members under the regular [representation rules](#representation):
- As a bound or other type-only constraint, `unknown` imposes no requirements.
- As a value, storage, or ABI type, bare `unknown` has no layout, so it erases to `Dynamic<unknown>` (and then can be narrowed and accessed through the runtime value it carries).

```ds
function parse(value: unknown): string {
    if (value is string) {
        return value; // narrowed by the guard
    }
    todo("...");
}

struct Event {
    payload: unknown; // erases to Dynamic<unknown>
}
```

For genuinely heterogeneous storage, `Dynamic<unknown>` is our "erased universal value": a fat pointer carrying the value and its runtime descriptor, introspectable via [reflection](#reflection) and narrowable (for runtime-discernible forms) via `is`.
By default, `Dynamic<T>` is a managed reference type (like a class), so it can be passed around, aliased, and mutated freely.

## String

Destack wants to be "TypeScript++", and thus we also follow JavaScript's string behavior: `string` length and positional access are _defined_ in terms of UTF-16 code units, and - just like TS's own string iterator - iteration yields Unicode code points (mapping to `char`).

```ds
const text = "héllo";

text.length satisfies usize; // UTF-16 code units, as in JS

for (const c of text) {
    c satisfies char; // iteration by code point, as in JS
}
```

Destack is stricter than TS for `string` indexing: `text[i]` gives a `char`, and indexing into a lone surrogate [traps](./expressions.md#panics) (a lone surrogate is virtually always a bug, and the views below are the explicit alternatives).
For the three main ways of looking at a string, the standard library provides explicit projections:

| View | Type | Meaning |
| --- | --- | --- |
| `text.units()` | `[uint16]`-shaped view | raw UTF-16 code units (JS's indexing model) |
| `text.chars()` | iterator of `char` | Unicode scalar values |
| `text.bytes()` | `[uint8]`-shaped view | UTF-8 bytes |

On native targets, `string` is immutable owned UTF-8 (the library's `String` literally holds `bytes: Unique<[uint8]>`); on JS/TS targets, strings are the host engine's strings.

## Intervals

Ranges in type position become an interval type over _bounded_ sets like `int`, `bigint`, or `char`; basically, an interval type is a static subset of its scalar type (e.g., `1..4` in type position is equivalent to `1 | 2 | 3`).
Assignments to interval-typed places must already have an interval-compatible type - the compiler does _not_ prove arithmetic expressions stay inside intervals and we do not insert implicit runtime checks for interval assignments.

```ds
type Digit = 0..=9;
type LowerAscii = 'a'..='z';
type UserPort = 1024..=65535;

let digit: Digit = 7;
let letter: LowerAscii = 'm';
let port: UserPort = 8080;

digit satisfies int;
letter satisfies char;
port satisfies int;
```

Interval types also compose with unions, aliases, and newtypes:

```ds
type HexDigit = 0..=9 | "a" | "b" | "c" | "d" | "e" | "f";
type NonZeroByte = 1..=255;
newtype Port = 1..=65535;
```

Intervals can also constrain static parameters:

```ds
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}
```

Because interval types are basically just aliases to union types, they must be _bounded_ sets, and as floating point are not bounded, plain `number`s cannot participate.
It follows that runtime values can become an interval type through ordinary _narrowing_ - good old [range patterns](./expressions.md#patterns) and [`is` checks](./expressions.md#guards):

```ds
newtype Port = 1..=65535;

function parsePort(n: int): Result<Port, ParseError> {
    match (n) {
        1..=65535 => Result.ok(Port(n)) // `n` is narrowed into the interval
        _ => Result.err(ParseError(`port out of range: ${n}`))
    }
}
```

## Newtypes

TypeScript is (primarily) structurally typed: an interface is satisfied by any value matching its shape, regardless of whether it explicitly `implement`s the interface.
However, sometimes explicit nominality is helpful for correctness and expressiveness, and Destack adds `newtype` as the nominal counterpart to `type`.
Like `type`, `newtype` follows its backing type: representable backings result in nominal concrete types, and constraint backings produce nominal constraints.

For example, with plain `type`s and aliases, there is no actual protection against accidental assignment.
(The TS ecosystem commonly resorts to "branding" hacks to work around this limitation.)
```ts
type UserId = number;
0 satisfies number; // OK, TS is happy, but ouch

type OrderTag = string;
"invalid" satisfies OrderTag; // OK, TS still happy, also ouch
```

```ds
newtype UserId = number;
const userId: UserId = 0; // ERROR: number is not UserId

newtype OrderTag = string;
const orderTag: OrderTag = "invalid"; // ERROR: string is not OrderTag
```

To construct a concrete newtype value, use explicit `T(..)` call syntax:

```ds
newtype UserId = number;
UserId(1) satisfies UserId;

newtype OrderTag = string;
OrderTag("tag") satisfies OrderTag;

newtype Point = (number, number);
Point(1, 2) satisfies Point;

newtype Rectangle = {
    start: Point;
    end: Point;
}
Rectangle({ start: Point(0, 0), end: Point(1, 1) }) satisfies Rectangle;

newtype AuthenticatedUser = User;
AuthenticatedUser(user) satisfies AuthenticatedUser;
```

Concrete newtypes are representation-transparent to the compiler but opaque to the type system.
Construction and projection across the backing boundary are explicit at the type level and of course "zero-cost" at runtime:

```ds
const id = UserId(1);
const raw = id as number;
```

## Newtype Interfaces

Newtype declarations add nominality to their backing type, and Destack also supports **nominal interfaces** using the `newtype` modifier on `interface` declarations.
(This makes newtype interfaces behave essentially like traits in other languages, no weird "branding tricks" required.)

```ds
// structural interface (standard TypeScript behavior)
interface Drawable {
    draw(): void;
}
const x: Drawable = { draw() {} };  // OK: structural match

// nominal interface (requires explicit `implements`)
newtype interface Add<T = this> {
    type Output;

    add(other: T): this.Output;
}
```

Nominal interfaces require **explicit `implements`** clauses, so structural compatibility alone doesn't satisfy the constraint (unlike with regular `interface`).
Newtype interfaces are used for explicit behavioral traits like [operator interfaces](./expressions.md#operators) (e.g., `Add`, `Compare`), and for [capability traits](./memory.md#capabilities) (e.g., `Copy`, `Clone`, and `SharedSafe`).
A `newtype interface` is nominal as a constraint, but it is still not a concrete value representation.

## Extensions

It is sometimes convenient to attach additional logic and data directly to a type, even and especially when the type is not defined locally (in the same module or even the same package).
Rust supports this with `impl` blocks (and only `impl` blocks, actually), and Destack supports _additional_ `extension`s to add instance and static members to any _nominal_ type:

```ds
class Vector2 {
    x: float32;
    y: float32;

    constructor(x: float32, y: float32) {
        this.x = x;
        this.y = y;
    }
}

// extension may be in a different file or package altogether
extension of Vector2 {
    static ZERO = new Vector2(0.0, 0.0);

    magnitude(): float32 {
        return (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

Extensions can be added to any **nominal type**, including `struct`, `class`, `enum`, and `newtype`, whether defined locally or in a foreign / imported module.
Plain type aliases (`type X = ...`) and structural types (`{ x: number }`) cannot receive extensions because it would be unclear when they should apply.

Extensions can also be named for and then referenced explicitly for export and import:

```ds
import { User } from "@/model/user";

export extension UserUtils of User {
    validate(): boolean {
        todo("...");
    }
}
```

The visibility of extension members follows from their placement:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension of int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils of Date { ... }`).

### Blankets

Extensions may declare their own generic parameters to extend generic types, and a generic extension over a whole family of instantiations is a "blanket" extension (once again, similar to Rust):

```ds
extension<T> of Box<T> {
    isEmpty(): boolean {
        // ...
    }
}

// conditional conformance: Box<T> is Show only when T is too
extension<T> of Box<T> implements Show where T: Show {
    show(): string {
        `Box(${this.value.show()})`
    }
}
```

The target of a blanket decides the scope of the claims it may make:

| Blanket | Example | Who may declare it |
| --- | --- | --- |
| Members over a bounded parameter | `extension Arithmetic<T: int> of T { ... }` | anyone |
| `implements` over a nominal application | `extension<T> of Box<T> implements Show` | anyone |
| `implements` over a bare bounded parameter | `extension<T: Equal> of T implements PartialEqual` | only the package declaring the interface |

Member blankets are lexical like all extension members, so the bound just names the candidate domain and nothing can surprise code that didn't import it.
(Unbounded targets - including the `T: unknown` spelling of the same thing - are rejected, because methods on _everything_ pollute every candidate set.)

An `implements` blanket makes a global claim, so it needs a clear owner: a nominal target gives the conflict surface one, while an open-domain claim over every type that ever satisfies a bound belongs to the contract's owner.
That last form is also how interface hierarchies ship their bridges anyway:

```ds
// in the package declaring PartialEqual
extension<T: Equal> of T implements PartialEqual {
    // ...
}
```

All `implements` blankets participate in the same general coherence rules: overlapping implementations of the same interface for the same type - including blanket-vs-specific overlap - are a program-wide error (see [Coherence](./expressions.md#coherence)).

## Enums

Enums are nominal aliases to a set of constants, just like in TypeScript, but in Destack, enums do _not_ implicitly cast to their backing type and explicit conversions are required for the backing value type.
Like other nominal types, enums can carry instance and static members, and of course can also receive extensions.

```ds
enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,

    static Default = Priority.Medium;

    label(): string {
        match (this) {
            Low => "low"
            Medium => "medium"
            High => "high"
        }
    }
}
```

## Tagged Unions

Discriminated unions are very convenient and fit well into existing TypeScript, but by themselves lack nominal containers (and items) to attach behavior to.
Using Destack's nominality via `newtype` and the builtin `Tagged` [`derive`](./expressions.md#derive), TypeScript's well known discriminated unions become even more ergonomic sum types:

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

extension of Shape {
    static DEFAULT = Shape.Rectangle({ width: 10, height: 20 });

    variant() {
        match (this) {
            Shape.Rectangle({ width, height }) => "rectangle"
            Shape.Circle({ radius }) => "circle"
        }
    }
}

// create values of tagged newtype unions with <Type>.<Variant>
const rectangle = Shape.Rectangle({ width: 10, height: 20 });
const circle = Shape.Circle({ radius: 5 });
```

The discriminant field is inferred from the union via the `Tagged` derive macro from the unique common field whose variants carry distinct literal values.
It behaves essentially just like builtin sugar that is expanded into a constructor function:

```ds
extension of Shape {
    // for each tagged variant, expose a "constructor" by tag
    static Rectangle({ width: int32, height: int32 }) {
        { kind: "rectangle", width, height }
    }

    // ...
}
```

Except that `Tagged` enums also work in pattern position, and - thanks to some compiler magic that wouldn't work in pure userland - the variant head behaves like a real variant pattern:

```ds
match (shape) {
    Shape.Rectangle({ width, height }) => width * height
    Shape.Circle({ radius }) => radius * radius
}
```

By default, string discriminants are exposed as `UpperCamelCase` constructor names - the other supported naming policies are:

| Tagged Casing | Example |
|--------|---------|
| `"preserve"` | `rectangle` → `rectangle` |
| `"camelCase"` | `rectangle_shape` → `rectangleShape` |
| `"UpperCamelCase"` | `rectangle_shape` → `RectangleShape` |
| `"snake_case"` | `RectangleShape` → `rectangle_shape` |
| `"SCREAMING_SNAKE_CASE"` | `RectangleShape` → `RECTANGLE_SHAPE` |

```ds
@derive(Tagged({ case: "preserve" }))
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

const rectangle = Shape.rectangle( /* ... */ );
const circle = Shape.circle( /* ... */ );
```

## Structs

Structs are nominal value types for data with a fixed shape, but without reference identity, constructors, or inheritance.
Basically, structs are just their values with a name attached, much like structs in other "systems-y" managed languages.
Structs are created via the usual `T { .. }` constructor form to distinguish them from regular objects, they do not and cannot have `new`-like `constructor`s of their own.

```ds
struct Point {
    x: float32;
    y: float32;
}

let x: Point = Point { x, y };  // OK
let x: Point = { x, y };        // ERROR: plain object is not Point
```

Like all type positions, struct expressions also support the `_` placeholder for contextual type inference:

```ds
let x: Point = _ { x, y };  // OK
```

Struct expressions can update an existing struct value with the familiar "spread" operator `...expr`, keeping the nominal struct type:

```ds
const moved = Point { ...x, y: 10.0 };
moved satisfies Point;
```

Plain object spreads can also read the fields of a struct value, but they produce a structural object rather than the nominal struct.

```ds
const object = { ...x, label: "origin" }; // x: Point
object satisfies { x: float32; y: float32; label: string };
```

Conversely, struct expressions can also spread _from_ object literal expressions (when the field set satisfies the struct):

```ds
const base = { x: 1.0, y: 2.0 };
const point: Point = _ { ...base };
```

Classes can also be spread into plain object expressions, following TypeScript's object-shaped model, but the result is structural and carries only fields.

## Classes

Classes follow the TypeScript-shaped model for managed objects with identity - except, of course, without the prototype chain or any dynamic or unsound shenanigans.
Also, class fields require every instance field to be initialized by its declaration or every constructor path.
(Optional fields do not need eager initialization, they default to `undefined`.)

```ds
class Counter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    increment(): int32 {
        this.value += 1;
        this.value
    }
}

const counter: Counter = new Counter(1);
counter.increment() satisfies int32;
```

Like structs, classes support the `_` placeholder for type inference:

```ds
const counter: Counter = new _(1);
counter.increment() satisfies int32;
```

Class methods are concrete (have only one implementation) by default and must be declared as `virtual` to enable _virtual_ dispatch of instances methods in subclasses.
Similarly, class methods are marked as `abstract` to _require_ an `override` in an implementing subclass such that the class can actually be constructed.

```ds
abstract class Logger {
    abstract write(message: string): void;

    virtual flush(): void {}
}
```

Additionally, classes may be marked `final` to prevent downstream classes from extending the declaration.

```ds
final class PacketHeader {
    length: uint32 = 0;
}
```

TypeScript visibility modifiers are fully supported, but the `#field` private syntax form is redundant and not allowed in `.ds` files (use `private` instead).

## Arrays, Slices and Tuples

Destack supports richer sequence forms beyond TypeScript's dynamic arrays - `T[]` / `Array<T>` with explicit slices, fixed arrays, and tuples.
Unfortunately, not much syntax was left here, so we had to adopt the slightly non-TS-y syntax forms of `[T]` and `[T; N]` for slices and fixed arrays, respectively.
Tuples use the native TS++ form `(A, B)` or the TypeScript-compatible array form `[A, B]`.

| Forms | Representation | Meaning |
|------|----------------|---------|
| `T[]`, `Array<T>` | Collection class | Growable, homogeneous, dense sequence with capacity |
| `[T]`, `Slice<T>` | Slice header | Pointer plus length, no capacity |
| `[T; N]`, `FixedArray<T, N>` | Inline array | Exactly `N` elements stored in the value |
| `(A, B)`, `[A, B]` | Inline product | Heterogeneous sequence of owned values |

Dynamic arrays are just class, regular managed objects with identity, while slices, fixed arrays, and tuples are value types (`struct`s, basically).
However, Destack does not permit holes in arrays or any other sequences, and indexing into `T[]` therefore always returns `T`.
Unlike in Rust, and somewhat more like in Go, Destack's `[T]` slice is a sized fat pointer and thus a first-class slice _value_.

```ds
let x: int32[] = [1, 2, 3];       // dynamic array of int32
let x: Array<int32> = x;          // dynamic array of int32

let x: [int32] = [1, 2, 3];       // slice of int32
let x: Slice<int32> = x;          // slice of int32

let x: [int32; 3] = [1, 2, 3];    // fixed array of int32
let x: FixedArray<int32, 3> = x;  // fixed array of int32

let x: (int32, int32, int32) = (1, 2, 3); // tuple of int32
```

By default, array literals are dynamic arrays but can coerce to other sequence forms in place when contextually required.
Fixed arrays are just a homogeneous sequence of values whose length is statically known (and part of the type): they are inline value/layout types by default, and definitionally cannot grow.
(If you need an array that can grow, use a dynamic array, i.e. `T[]` / `Array<T>`)

```ds
type Block = [uint8; 4096]; // 4KB of uint8
type Vec3 = [float32; 3];   // 3 float32s

let rgb: [uint8; 3] = [255, 128, 0];
let zeroes: [uint8; 32] = [0; 32];
```

Fixed arrays and slices also work directly in patterns: fixed array patterns already know their length statically, while slice patterns can use a rest binding for the tail / head:

```ds
declare const rgb: [uint8; 3];
declare const bytes: [uint8];

let [r, g, b] = rgb;

match (bytes) {
    [0x89, 0x50, 0x4e, 0x47, ...rest] => parsePng(rest)
    [0xff, 0xd8, ...rest] => parseJpeg(rest)
    _ => Result.err("unknown image format")
}
```

Tuples are fixed heterogeneous products, and of course also work as patterns.
The formatter preserves whether a type used the parenthesized or array form.
Because `[T]` is a slice, a singleton array-form tuple requires its trailing comma: `[T,]`.

```ds
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

One-element tuples use a trailing comma, and empty tuples are `()` or `[]` according to their form.
Since tuples are also just value containers, empty tuples occupy no space.

```ds
type One = (int32,);
const one: One = (1,);
const empty: () = ();

type ArrayOne = [int32,];
type ArrayEmpty = [];
```

## Readonly

TypeScript's `readonly` is shallow, while in `.ds`, `readonly T` is always a _deep_ read-only view of `T`.
That is, `readonly T` forbids _any_ mutation through its `T`, including via nested members, and reads through a readonly view never _widen_.

```ds
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
    tags: string[];
}

declare const user: readonly User;

user.profile.name = "Grace"; // ERROR: readonly view
user.tags[0] = "admin";      // ERROR: readonly view
```

## Generics

Destack supports classic TypeScript-shaped generics: inference, constraints, defaults, conditional types, mapped types, indexed access types, and the rest of the usual machinery.
Unlike in TypeScript, however, Destack's parameters can also represent _values_ that are then substituted into expressions _and_ both values and types are available during inference.

To distinguish static value parameters from static type parameters (and literal value types), we use the `comptime` modifier on the generic parameter declaration (akin to Rust's `const` modifier, alas this was already taken in TypeScript):

```ds
type Buffer<comptime N: uint> = [uint8; N];
```

Generic bounds can use the cleaner `<T: Constraint>` form, just like dynamic parameters.
Unlike with `comptime <expr>` (discussed later), the `comptime` modifier merely means that `N` is a generic value parameter that has to be evaluatable as a static term during inference.

```ds
function copy<T, comptime N: uint>(src: [T; N]): [T; N] {
    let dst: [T; N];
    for (let i = 0; i < N; i++) {
        dst[i] = src[i];
    }
    dst
}
```

As in TypeScript, type parameters may include a `const` modifier to retain "fresh" precision in arguments.
To be clear, `const T` is about type widening, and `const T` parameters are still _type_ parameters (unlike `comptime N`), and they're also the reason we had to use a different syntax for true value parameters in the first place.

```ds
declare function freeze<const T>(value: T): T;

const value = freeze({ kind: "ready", level: 1 });
value.kind satisfies "ready";
```

Generic parameters also support `...` forms:

```ds
type Callback<...Parameters, Return> = (...parameters: Parameters) => Return;
```

Type inference (including generics) works across modules, even when modules circularly reference one another - though of course, this should be used with caution and can lead to longer compile times because it forces large connected components during inference checking.

## Variance

Variance describes how subtyping relates generic types, including the types that managed language users may not usually think of as "generic" (like `Array` or `Record`).
Mutable covariance - the fact that TypeScript lets us assign `Circle[]` to `Shape[]` and then mutate the `Circle[]` _through_ the widened `Shape[]` alias - is one of TypeScript's best known soundness holes, so Destack derives variance from one principle: **a position is invariant exactly when a widened value and the original can reach the same mutable storage.**

Widening compiles to nothing.
An implicit conversion that must rewrite the representation - injecting a value into a tagged union, erasing behind `Dynamic<T>`, widening a numeric literal into a concrete carrier - only happens where a fresh value materializes: an assignment, an argument, a return.
Inside an existing value there is no site to convert at, so type arguments only relate along **identity-witnessed** edges:

```ds
declare const circles: Holder<Circle>;

const shapes: Holder<Shape> = circles;           // OK: class upcasts change nothing physical
const either: Holder<Circle | Square> = circles; // ERROR: the payload would need a union tag
const boxed: Holder<unknown> = circles;          // ERROR: the payload would need an existential box
```

The rejected widenings still flow value by value: constructing `Holder<Circle | Square>` from a `Circle` tags the payload at the construction, and rebuilding an existing value arm by arm converts each payload at its own value position.

The memory form of a handle decides how much of this its payload needs:

| Handle | Payload arguments | Reason |
| --- | --- | --- |
| managed `T` | by derived variance under aliasing | other writable aliases persist |
| owned `^T` | by derived variance, storage covariant | a move leaves no alias behind |
| `readonly T`, `&readonly T` | by derived variance, storage covariant | the view strips every write path, deeply |
| `&T`, `&exclusive T` | exact | writes flow through the borrow |
| `*T` | exact | raw pointers promise nothing |

Placement (`local`/`shared`) is orthogonal to ownership and plays no variance role.

For generic declarations, variance is derived per parameter from usage.
Storage drives it: a readonly field is covariant, a mutable field is invariant under aliasing and covariant for owned copies and readonly views (container elements and managed payloads stay reachable through other aliases even from owned copies, so only a readonly view relaxes them).
Methods split by dispatch:

- **Class and interface definition methods are carried.** A constructed instance travels with its own method instantiations, so a widened handle would execute code compiled at the old arguments. Definition methods therefore constrain every handle context by their function-position variance, owned moves included.
- **Extension methods and value-type methods dispatch statically.** Every call instantiates at the receiver's static type, so nothing stale is carried and they never constrain variance - the same rule as Rust, where only fields drive variance and `impl` blocks do not.

```ds
class Source<T> { take(): T }                      // T only comes out -> covariant
class Sink<T>   { put(value: T): void }            // T only goes in   -> contravariant
class Pipe<T>   { take(): T; put(value: T): void } // both             -> invariant, even as ^Pipe<T>
```

A userland collection becomes view-covariant the same way: keep the fields in the class and the methods in extensions.
`Array<T>` itself is written this way, so a mutable array is invariant while a readonly view widens:

```ds
declare const circles: Circle[];

const shapes: Shape[] = circles;        // ERROR: mutable arrays are invariant
const view: readonly Shape[] = circles; // OK: readonly views are covariant
const copies: Shape[] = [...circles];   // OK: explicit copy reifies Shape elements
```

Declared variance closes the gaps derivation cannot reach.
A generic parameter that never occurs in its declaration is an error (`EC442`); an explicit modifier like `out T` keeps a deliberate marker parameter.
Declared modifiers on derivable declarations are checked against usage - `class Evil<out T> { slot: T }` is rejected (`EC443`) because the mutable field uses `T` invariantly.
On an intrinsic newtype the compiler cannot derive anything, so the modifier is a trusted assertion about the opaque storage, composed with the handle context like a field: `Unique<out T>` is covariant for owned and viewed storage and still invariant behind a writable alias.

For the other ways to write "a collection of shapes", the element representation decides everything:

| Element type | `Shape[]` means | Holds |
| --- | --- | --- |
| `class Shape` | array of managed references | any subclass, open set |
| `type` / `newtype` union | array of tagged variant layouts | the listed variants, closed set |
| `interface Shape` | array of `Dynamic<Shape>` | any implementor, open set |
| `Dynamic<Shape>` | array of erased fat pointers | any implementor, open set |

An array of `class Shape` holds subclasses because upcast references are physically identical; an array of a union holds exactly the listed variants because each element carries the union layout.
The same reasoning as everywhere else: representation-changing widenings happen at value positions, never inside storage.

## Static

Unlike TypeScript, Destack actually _compiles_, so we need to figure out during "compile time" the final type of each value and fill in values for all the known constants.
To do this, we need to consider three different "evaluation times" between the source code and the actual executable:

| World | Meaning | Example |
|-------|---------|---------|
| Static | types, values, and relations known while checking | `T`, `N`, `this.Width`, `T extends string? A : B` |
| Comptime | ordinary code explicitly evaluated by the compiler | `comptime factorial(10)` |
| Runtime | ordinary program execution | `readFile(path)`, `worker.postMessage(msg)` |

Type inference and static term evaluation are one and the same, which is why we get both fast generics and powerful type evaluation using **static terms**: a small subset of the language (like TypeScript type operators) available during type inference.
All dynamic `comptime <expr>` _execution_ happens _after_ type inference, so it can do anything that runtime code can do.
This keeps compilation fast and predictable, and thanks to TypeScript's flexible type algebra, static terms are still pretty powerful:

| Input | Example |
|-------|---------|
| Type parameters | `T` |
| Static value parameters | `N` in `function f<comptime N: uint>()` |
| Type aliases and generic applications | `Buffer<N>`, `Payload<T>` |
| Associated types and constants | `I.Item`, `Register.Width` |
| Static `const`s and imports | `N` imported from `./a.ds` |
| Enum members and nominal constants | `OperatingSystem.Windows` |
| Literal values | `4`, `"shared"`, `true` |
| Static operators | `N * 2`, `Mode == "inline"` |
| Contextual type form | `PlaceOf<this>` inside a type declaration |
| Module and profile metadata | `import.meta.platform` |
| Type operators | `keyof T`, `T[K]`, `T extends string`, `T implements I` |
| Layout intrinsics | `sizeOf<T>()`, `alignOf<T>()` |

Static terms are required wherever the language needs an inference-known answer: fixed array lengths, conditional types, associated members, static decorators, [layout queries](#layout), and [placement algebra](./memory.md#algebra).
Type inference may flow _out_ of modules, but Destack does not support circular static inference or inference across modules in any way.

```ds
type Block<comptime N: uint> = [uint8; N];
type Payload<T> = T extends string ? Utf8Payload : BinaryPayload;
type BufferIndex<Mode> = Mode == "inline" ? InlineIndex : ExternalIndex;

struct Buffer<T, comptime Mode: "inline" | "external"> {
    index: BufferIndex<Mode>;
    data: T[];
}
```

In the `Buffer` example, `Mode` is carried as a generic value and used by a conditional type.
The declaration still has one checked field named `index`; only the field type changes by instantiation.
Generic-dependent static terms may select types, constants, layouts, and overloads, but they do not remove source nodes from a generic declaration.
That keeps generic declarations checked once, Rust-style, instead of turning generic instantiation into template expansion.

```ds
function size<comptime Wide: boolean>(): Wide extends true ? 8 : 4 {
    if (comptime Wide) {
        return 8;
    } else {
        return 4;
    }
}
```

## Associated Types and Constants

Associated types and constants contribute static members to a type that can be reused within the type and its implementors but do not need to be exposed to every single caller.
Both associated types and constants also work in abstract types, and as they are associated with the type directly, they do not occupy any instance space on the type.
All statically known types and constants share the same static evaluation logic, and thus associated types and constants also mix with generic parameters, conditional types, decorators, and so on.

```ds
newtype interface Collection {
    type Item;
    type Return = void;

    next(): IteratorResult<this.Item, this.Return>;
}

function collect<I: Collection>(iter: I): I.Item[] {}
```

Associated types are type aliases scoped to some struct, class, or interface and can also reference the owner's generic parameters.

```ds
interface BufferPool {
    type Buffer<T>;
    type Error;

    acquire<T>(count: usize): Result<this.Buffer<T>, this.Error>;
    release<T>(buffer: this.Buffer<T>): void;
}
```

Associated types can have their _own_ generic parameters with the same generic parameter forms as ordinary declarations, including type parameters and `comptime` value parameters (these are generic associated types, often called GATs).

```ds
interface Storage {
    type Handle<T>;
    type Page<comptime Size: uint>;
}

struct SharedStorage implements Storage {
    type Handle<T> = shared StorageHandle<T>;
    type Page<comptime Size: uint> = shared StoragePage<Size>;
}
```

In addition to associated types, nominal type declarations also support associated constant members as static compile-time values.
Like `static` members, `comptime const`s require no instance storage, but unlike `static` members, `comptime const`s are statically evaluated _during_ compilation.

```ds
interface RegisterBlock {
    comptime const Width: uint;

    read(): [uint8; this.Width];
    write(bytes: &[uint8; this.Width]): void;
}
```

Associated members participate in the same static evaluation / inference world, and so associated members can express dependent types and values that are dependent on others (including inferred!).

```ds
interface Matrix<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Bytes = [uint8; this.Width];
}
```

Associated members (both types and constants) can be refined explicitly at application sites whenever an erased or constrained value needs a concrete associated surface with `type Name = T` for types and `comptime Name = value` for constants.

```ds
declare function readBlock<T: RegisterBlock<comptime Width = 16>>(block: T): [uint8; 16];
```

## Constraints

Sometimes, defining the constraints and relations for type parameters can become unwieldy or outright impossible with only type annotations on each individual term.
Destack supports explicit (type-space) `where` clauses to define additional constraints for complex types and signatures, very much like Rust:

```ds
function merge<T: int, U>(): T where U: Comparable<T> {
    // ...
}
```

A `where` clause accepts the same constraint forms as inline bounds, plus a few relations that inline bounds cannot otherwise express:

| Form | Example | Meaning |
| --- | --- | --- |
| Interface bound | `T: Comparable<U>` | a parameter must satisfy a constraint |
| Associated member bound | `I.Item: Display` | an associated type must satisfy a constraint |
| Equality constraint | `T.Output == U` | two static terms must normalize to the same type or value |

## Representation

Destack's general philosophy is to let users opt _in_ to additional control and complexity as needed - as much as possible, things should "just work" like in TypeScript.
That said, the actual memory representation of types does matter, and our unique blend of TypeScript type algebra and actual systems-y AOT compilation means that Destack needs to make representation tradeoffs differently from other languages.
Specifically, we distinguish four type properties that affect representation:

| Axis | Question | Examples |
| --- | --- | --- |
| *nominal* vs *structural* | must the type be constructed or implemented explicitly? | `newtype` / `struct` / `class` / `newtype interface` vs `type` / `{ x: number }` |
| *transparent* vs *opaque* | does the name just expand to a type expression? | `type` vs `interface` / `struct` / `class` / `newtype` |
| *closed* vs *open* | does the type expression close to one represented shape? | object alias / union alias vs interface / open index signature |
| *represented* vs *erased* | how is a value slot stored? | direct layout vs `Dynamic<T>` |

The position a type is used in decides who "commits" to a representation: a _constraint_ position leaves the choice to each use site, while a _storage_ position needs an actually concrete ("storable") representation:

| Form | Type behavior | Storage default |
| --- | --- | --- |
| `type Point = { x: number; y: number }` | Structural, transparent, closed when reified. | Anonymous object layout. |
| `interface Point { x: number; y: number }` | Structural, named, open. | `Dynamic<Point>` |
| `newtype interface Point { x: number; y: number }` | Nominal constraint, open. | `Dynamic<Point>` |
| `type Shape = Circle | Rectangle` | Transparent, closed. | Represented union. |
| `type Flags = Record<"debug" | "trace", boolean>` | Transparent, closed when reified. | Anonymous object layout. |
| `type Bag = Record<string, Handler>` | Transparent, open index constraint. | `Dynamic<Bag>`. |
| `struct Point { x: number; y: number }` | Nominal, closed. | Inline value. |
| `class Point { x: number; y: number }` | Nominal, closed. | Managed reference. |
| `newtype Point = T` | Nominal, closed when `T` is closed. | Newtype representation. |
| `<T: PointLike>` | Explicit generic parameter. | Concrete per instantiation. |

```ds
type Point = {
    x: int32;
    y: int32;
};

function draw(point: Point): void {
    // ...
}

struct Rectangle {
    position: Point;
}

draw({ x: 1, y: 2, z: 3 }); // OK

const rectangle = Rectangle {
    position: { x: 1, y: 2, z: 3 } // ERROR: fresh object literal has excess property `z`
};
```

Transparent aliases over closed represented types can be represented directly in storage positions, while open constraints and interfaces become `Dynamic<T>` in storage positions (unless the user writes an explicit generic parameter):

```ds
interface PointLike {
    x: int32;
    y: int32;
}

function draw(point: PointLike): void {
    // ...
}

struct Rectangle {
    start: PointLike;
    end: PointLike;
}
```

Desugared, that is basically:

```ds
struct Rectangle {
    start: Dynamic<PointLike>;
    end: Dynamic<PointLike>;
}
```

For static storage, spell the generic parameters explicitly:

```ds
struct Rectangle<TStart: PointLike, TEnd: PointLike> {
    start: TStart;
    end: TEnd;
}
```

For contrast, a function-valued field is still an ordinary representable structural shape, because the field itself has a representation (a function pointer):

```ds
type WriterField = {
    write: (bytes: &[uint8]) => Result<usize, Error>;
};
```

Structural type annotations still typecheck structurally, and structural value expressions synthesize concrete anonymous shapes when they are used as values.
Here the annotation checks the object shape, and the object expression supplies the concrete value shape:

```ds
const point: { x: int32; y: int32 } = { x: 1, y: 2 };
```

Return positions are also considered storage positions, and the return representation is solved from the function body:
- When all joined return paths produce _one_ concrete type, the return is existential, that is, callers type against the declared transparent type, but the compiled function returns the specific concrete representation at zero cost.
- When the paths join _different_ concrete types and the declared return is a transparent data shape (like a union alias), the return reifies that shape's concrete layout - for a union alias, the tagged variant layout - just as a stored field would.

```ds
type Shape = Rectangle | Circle; // transparent alias

function foo(): Shape {
    return Rectangle(); // existential: the compiled return type is Rectangle
}

function bar(): Shape { // reified: the compiled return type is the Shape variant layout
    if (getRandom() > 4) {
        return Circle();
    } else {
        return Rectangle();
    }
}
```

A representation change is also exactly what separates a compiled conversion from a plain type-level widening.
An implicit conversion reifies as a coercion only when the identity function is not a valid witness for it: injecting a value into a tagged union writes a tag and erasing behind `Dynamic<T>` builds the fat pointer, while readonly and variance widenings change nothing physical and compile to nothing.
Literals never convert at all; a constant materializes directly at its solved type.

## Layout

Layout is the concrete storage and ABI shape selected for a representable type under the active target, the default representation being `@repr("destack")`.
Only represented types have layout; structural shapes become represented when a value expression materializes a concrete object, while open slots use an explicit generic parameter or become indirect behind `Dynamic<T>`.
The exact layout of a type can be configured via decorators that constrain its representation as needed, the conventions being very similar to Rust's:

| Decorator | Meaning |
| --- | --- |
| `@repr("destack")` | Use the native Destack representation. |
| `@repr("C")` | Use the active target's C ABI layout. |
| `@repr("transparent")` | Give a single-field declaration the same ABI representation as its field. |
| `@repr(T)` | Use primitive scalar `T` as an enum backing representation. |
| `@repr({ align: N })` | Raise the minimum aggregate alignment to `N`. |
| `@repr({ packed: true })` / `@repr({ packed: N })` | Lower the maximum field alignment, with `true` equivalent to `1`. |

```ds
@repr({ align: 64 })
struct CacheLine {
    value: uint64;
}

@repr("C", { packed: true })
struct WireHeader {
    tag: uint8;
    size: uint32;
}
```

Destack also supports querying parameters of the effective representation during compilation - available as a [static term](#static) during inference - for conditional branching and storage:

| Layout Query | Result |
| --- | --- |
| `sizeOf<T>()` | The byte size of `T` as `usize`. |
| `alignOf<T>()` | The required alignment of `T` as `usize`. |
| `strideOf<T>()` | The spacing between adjacent array elements of `T` as `usize`. |
| `layoutOf<T>()` | The reflected `size`, `align`, `stride`, and shape for `T` as a `Layout` value. |

Layout queries evaluate as type operations: a query over an open or generic type stays symbolic and reduces to its value once its argument becomes concrete.
Inference therefore answers exactly the queries whose inputs it has settled, and comptime evaluation forces the remainder.
Layout is never stored in the compiler's program representation; every phase recomputes it on demand from the type and the active target through one shared measure, and code generation derives its structural layout from that same measure.

## Reflection

TypeScript types are - by design - erased at runtime, which means we can't easily perform runtime type checks or any meaningful reflection.
Destack supports type reflection both at runtime and at compile time with `Type<T>` as a normalized view.
Dynamic type expression can be turned into its reflected type with (implicit or explicit) casting to its `Type` representation:

```ds
struct User {
    name: string;
    age: uint;
}

let user: User = User { name: "Alice", age: 30 };

const UserType: Type<User> = User;     // implicit cast
const UserType = Type.of<User>();      // explicit, same thing
```

This also works for generic APIs that operate on types as static values:

```ds
function parse<comptime T: Type>(raw: string): T {
}

const user = parse<User>("...");
```

```ds
const userSize = comptime sizeOf<User>();
const requestLayout = comptime layoutOf<Request<Body>>();
type InlineBytes<T: Concrete> = [uint8; sizeOf<T>()];
```
