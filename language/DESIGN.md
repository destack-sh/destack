# Overview

The Destack language (`.ds`) is a superset of "strict modern" TypeScript with support for `.ts` and `.tsx` files, true native AOT compilation and a fully integrated toolchain, _and_ it can also "compile" nicely to standard JS/TS targets.
Strict TypeScript code "just works", but Destack has absolutely **no JavaScript or NPM interoperability**  (see [COMPARISON.md](COMPARISON.md)).

We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated stack, and thus by "language" ("TypeScript++") we mean much more than "just" a coding language: a language, a runtime, a toolchain, plugins, and ultimately, a way of programming.

## Universality

We're very early in software, and we're still figuring out how to build optimal, correct, and integrated software systems.
Over 50 years, we have grown more and more layers of software sediment and need ever _more_ tools to get any code out the door, and yet confidence and performance have plummeted.
We can do better, but not by adding _more_ and more inscrutable pieces.

The best possible stack must be fully integrated across the language itself, the toolchain, the runtime, and basically anything that touches the code.
Only TypeScript is close to being a universal software foundation, because it runs directly on the web, and the web is the most ubiquitous software platform.
The TypeScript ecosystem has good - if not perfect - answers to all modern software needs, from great developer tools to rich interactive frontends to quite performant backends.

Excluding the legacy JavaScript baggage and all the dynamic prototype mess, modern TypeScript is surprisingly close to a fully AOT-compilable language (and most browsers retrofit compilation internally already based on these assumptions).
Embracing TypeScript and "the web ecosystem" lets us build a new toolchain that truly covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be completely free of JS overhead and (some) historic baggage.

# Language

"TypeScript++" is a superset of the "strict modern" subset of TypeScript, which essentially means that existing TypeScript (and TSX!) _just works_ **if** it follows our strict TypeScript-based type system _and_ has no exceptions.
Fortunately, strict TypeScript is already a best practice, and it's what you get when enabling the recommended soundness flags in TSC (mostly).
TypeScript++ adds some new features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do for frontend-shaped software, but for the entire software stack including "systems software".

There are solid arguments that a language should be minimal (like Zig or Go or even C), but we do not believe "language minimalism" to be pragmatic for the universal language and toolchain we want.
That said, TypeScript is already not a simple language, and any additional language features risk becoming unwieldy.
We embrace this tradeoff, and as we needed _some_ additions for serious systems programming, we took the opportunity to round out the language with modern ergonomics like patterns, operator overloading, reflection, and comptime.

## Types

Destack extends TypeScript's type system with precise primitives, nominal types ("`newtype`s"), value types ("`struct`s"), tuples, ergonomic constraints, and some additional niceties.

### Primitives

Destack is based on TypeScript, and TypeScript inherits its main primitive types from JavaScript: `string`, `boolean`, `number`, `bigint`, and `symbol`, plus the `null` and `undefined` sentinels.
It should be noted that `string` and `bigint` are not really special in Destack, they are just aliases to the standard library `String` and `BigInt` classes, respectively.
We forbid imprecise top types like `object` and `any`, and provide additional precise primitive types:
- precise numeric types beyond `number`, with variable-width signed and unsigned integers (`int8`, `uint32`, `int17`) as well as modern concrete float formats (`float16`, `bfloat16`, `float32`, `float64`)
- pointer-sized integers, i.e. integers as wide as the target pointer size, spelled `isize` and `usize`
- `int` and `uint` as aliases to `int64` and `uint64`
- `number` as an alias for `float`, and `float` as an alias to `float64`
- `char` as a single Unicode scalar value, distinct from `string`

`float16` is the IEEE-754 binary16 format, while `bfloat16` is a distinct 16-bit format intended for ML and tensor-heavy workloads.

Following the spirit of TypeScript's widening rules, numeric literals start as exact values and can flow into any numeric type that can represent them.
When no specific numeric context fits, the literals widen as usual to plain `number` (i.e. `float64`).

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

### Unknown

TypeScript has two "top" types: `unknown` and `any`, these types can contain all other types.
Of course, `any` is unsound, because everything can be assigned to and from it without any checks, so Destack only supports `unknown`.
Because `unknown` is just a transparent constraint, it behaves like an interface with zero members under the regular [representation rules](#representation):

- In constraint positions, `unknown` induces an implicit generic: `function parse(value: unknown)` behaves like `function parse<T>(value: T)` where the body knows nothing about `T` until it narrows.
- In storage positions, there is nothing to reify, so storing `unknown` induces a generic parameter just like storing an interface does.

```ds
function parse(value: unknown): string {
    if (value is string) {
        return value; // narrowed by the guard
    }
    todo("...");
}

struct Event {
    payload: unknown; // induces Event<T>, monomorphized per payload type
}

struct ErasedEvent {
    payload: Dynamic<unknown>; // one fixed erased representation
}
```

For genuinely heterogeneous storage, `Dynamic<unknown>` is the explicit erased universal value: a fat pointer carrying the value and its runtime type witness, introspectable via [reflection](#reflection) and narrowable via `is`.

### String

Destack aims to be "TypeScript++", and thus we inherit JavaScript's string behavior so the same code runs transparently on TS targets and native: `string` length and positional access are _defined_ in terms of UTF-16 code units, and - just like TS's own string iterator - iteration yields Unicode code points (mapping to `char`).

```ds
const text = "héllo";

text.length satisfies uint; // UTF-16 code units, as in JS

for (const c of text) {
    c satisfies char; // iteration by code point, as in JS
}
```

Destack is stricter than TS for `string` indexing: `text[i]` gives a `char`, and indexing into a lone surrogate [traps](#panics) (a lone surrogate is virtually always a bug, and the views below are the explicit alternatives).
For the three main ways of looking at a string, the standard library provides explicit projections:

| View | Type | Meaning |
| --- | --- | --- |
| `text.units()` | `[uint16]`-shaped view | raw UTF-16 code units (JS's indexing model) |
| `text.chars()` | iterator of `char` | Unicode scalar values |
| `text.bytes()` | `[uint8]`-shaped view | UTF-8 bytes |

On native targets, `string` is immutable owned UTF-8 (the library's `String` literally holds `bytes: Unique<[uint8]>`); on JS/TS targets, strings are the host engine's strings.

### Intervals

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

Because interval types are basically just aliases to union types, they are _bounded_ sets, and floating point numbers cannot participate.
(It would not make sense to have `0.0..1.0` since that would be inviting a whole new class of refinement types that we wanted to avoid in favor of more explicit and flexible nominality.)
It follows that runtime values can become an interval type through ordinary _narrowing_ - good old [range patterns](#patterns) and [`is` checks](#guards):

```ds
newtype Port = 1..=65535;

function parsePort(n: int): Result<Port, ParseError> {
    match (n) {
        1..=65535 => Result.ok(Port(n)) // `n` is narrowed into the interval
        _ => Result.err(ParseError(`port out of range: ${n}`))
    }
}
```

### Newtypes

TypeScript is structurally typed: an interface is satisfied by any value matching its shape, regardless of whether it explicitly `implement`s it.
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
Construction and projection across the backing boundary are both explicit and zero-cost:

```ds
const id = UserId(1);
const raw = id as number;
```

### Newtype Interfaces

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

Nominal interfaces require **explicit `implements`** declarations - structural compatibility alone doesn't satisfy the constraint, unlike for regular `interface`.
Newtype interfaces are used for explicit behavioral traits like [operator interfaces](#operators) (e.g., `Add`, `Compare`), and for [capability traits](#capabilities) (e.g., `Send`, `Sync`, `Copy`, and `Clone`).
A `newtype interface` is nominal as a constraint, but it is still not a concrete value representation.

### Extensions

It is sometimes convenient to attach additional logic and data directly to a type, even and especially when the type is not defined locally.
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

#### Blankets

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

All `implements` blankets participate in the same general coherence rules: overlapping implementations of the same interface for the same type - including blanket-vs-specific overlap - are a program-wide error (see [Coherence](#coherence)).

### Enums

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

### Tagged Unions

Discriminated unions are very convenient and fit well into existing TypeScript, but by themselves lack nominal containers (and items) to attach behavior to.
Using Destack's nominality via `newtype` and the builtin `Tagged` [`derive`](#derive), TypeScript's well known discriminated unions become even more ergonomic sum types:

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

The discriminant field is inferred from the union via a regular userland `Tagged` macro from the unique common field whose variants carry distinct literal values.
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

### Structs

Structs are nominal value types for data with a fixed shape, but without reference identity, constructors, or inheritance.
Basically, structs are just values with a name, much like structs in other "systems languages": an alias to the struct's components (with a certain layout and padding).
Structs are created via the usual `T { .. }` constructor form to distinguish them from regular objects (no constructors).

```ds
struct Point {
    x: float32;
    y: float32;
}

let x: Point = Point { x, y };  // OK
let x: Point = { x, y };        // ERROR: plain object is not Point
```

Structs also support the `_` placeholder for type inference:

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

Conversely, struct expressions can also spread from object literals (when the final field set satisfies the struct):

```ds
const base = { x: 1.0, y: 2.0 };
const point: Point = _ { ...base };
```

Classes don't get to participate in spreads because they carry identity, behavior, and constructors, and that would just be a confusing mess.

### Classes

Classes follow the TypeScript-shaped model for managed objects with identity, except of course without a prototype chain or any dynamic class shenanigans.
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
Similarly, class methods are marked as `abstract` to require an override before the class can be constructed.

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

### Arrays, Slices and Tuples

Destack supports richer sequence forms beyond TypeScript's dynamic arrays - `T[]` / `Array<T>` with explicit slices, fixed arrays, and tuples.
Unfortunately, not much syntax was left here, so we had to adopt the slightly non-TS-y syntax forms of `[T]` and `[T; N]` for slices and fixed arrays, respectively.
(This is also why `.ds` does not support `.ts`-style array tuples `[A, B]` and tuples in `.ds` must always be explicit `(A, B)`)

| Forms | Representation | Meaning |
|------|----------------|---------|
| `T[]`, `Array<T>` | Collection class | Growable, homogeneous, dense sequence with capacity |
| `[T]`, `Slice<T>` | Slice header | Pointer plus length, no capacity |
| `[T; N]`, `FixedArray<T, N>` | Inline array | Exactly `N` elements stored in the value |
| `(A, B)` | Inline product | Heterogeneous sequence of owned values |

Dynamic arrays are regular managed objects with identity, while slices, fixed arrays, and tuples are value types (`struct`s, basically).
However, Destack does not permit holes in arrays or any other sequences, and indexing into `T[]` therefore always returns `T`.
Unlike in Rust, and somewhat more like in Go, Destack's `[T]` is sized and a first-class slice _value_.

```ds
let x: int32[] = [1, 2, 3]; // dynamic array of int32
let x: Array<int32> = [1, 2, 3]; // dynamic array of int32

let x: [int32] = [1, 2, 3]; // slice of int32
let x: Slice<int32> = [1, 2, 3]; // slice of int32

let x: [int32; 3] = [1, 2, 3]; // fixed array of int32
let x: FixedArray<int32, 3> = [1, 2, 3]; // fixed array of int32

let x: (int32, int32, int32) = (1, 2, 3); // tuple of int32
```

By default, array literals are dynamic arrays but can coerce to our "fixed array" as needed.
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

For tuples, as said above, we still parse the "array tuple" syntax like `[number, string]` in non-`.ds` files, but require explicit tuple syntax like `(number, string)` in `.ds`.
Tuples are fixed heterogeneous products, and of course also work as patterns:

```ds
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

One-element tuples use a trailing comma, empty tuples are just `()`.
Since tuples are also just value containers, empty tuples occupy no space.

```ds
type One = (int32,);
const one: One = (1,);
const empty: () = ();
```

### Readonly

TypeScript's `readonly` is shallow, while in `.ds`, `readonly T` is always a _deep_ read-only view of `T`.
That is, `readonly T` forbids _any_ mutation through its `T`, and `readonly T` cannot be assigned to `T`, including via nested members.

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

As in TypeScript, `readonly` is a type-level access promise.
It does not freeze the runtime value.

### Generics

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

Dynamic parameters may _also_ be marked `comptime` when the caller should pass an ordinary argument expression that is still required to be evaluatable as a static term during compile time, mostly as a readability affordance where spelling the value as a generic argument would be awkward or constraining.

```ds
function repeat<T>(value: T, comptime count: uint): [T; count] {
    // ...
}

const values = repeat("x", 3);
values satisfies [string; 3];
```

Like dynamic parameters, Destack's generic parameters also support `...` forms:

```ds
type Callback<...Parameters, Return> = (...parameters: Parameters) => Return;
type Buffer<comptime ...Shape: readonly usize[]> = TensorBuffer<...Shape>;
```

Generic type inference - like all type inference - is local and flows "outward": each module infers from its own declarations and imports, downstream modules can use what it exports, and downstream uses can never feed back into upstream inference (unlike in TypeScript, mostly).

```ds:a.ds
declare function length<T, comptime N: uint>(xs: [T; N]): N;

export const RGB: [uint8; 3] = [255, 128, 0];
export const N = length(RGB);

N satisfies 3;
```

```ds:b.ds
import { N } from "./a.ds";

declare function double<comptime N: uint>(): N * 2;

export const M = double<N>();

N satisfies 3;
M satisfies 6;
```

### Variance

Variance describes how typing and subtyping relations work for generic types, including for all the types that managed language users may not even usually think of as generic (like `Array`).
Mutable covariance - the fact that you can assign `Circle[]` to `Shape[]` and then mutate `Circle[]` _through_ the widened `Shape[]` alias - is one of TypeScript's best known soundness holes and a classic footgun.
This generalises 
Because Destack needs to be sound, we only support this sort of widening when it is unambiguously safe:

| Position | Variance | Example |
| --- | --- | --- |
| Readonly positions | covariant | `readonly Circle[]` is assignable to `readonly Shape[]` |
| Mutable storage positions | invariant | `Circle[]` is _not_ assignable to `Shape[]` |
| Function parameters | contravariant | `(shape: Shape) => void` is assignable to `(circle: Circle) => void` |
| Function returns | covariant | `() => Circle` is assignable to `() => Shape` |

For generic types, variance is derived per parameter from their members usage (like in TypeScript): a parameter that only comes _out_ (returns, readable fields) is covariant, one that only goes _in_ (parameters, writable fields) is contravariant, and one that does both - a mutable field counts as both at once - is invariant.

```ds
class Source<T> { take(): T }                      // T only comes out -> covariant
class Sink<T>   { put(value: T): void }            // T only goes in   -> contravariant
class Pipe<T>   { take(): T; put(value: T): void } // both             -> invariant
```

And because `Array<T>` is just a regular (well known) standard library type, and it has both readable and writable positions for its generic parameter, `Array<T>` is invariant in `T`:

```ds
declare const circles: Circle[];

const shapes: Shape[] = circles;        // ERROR: mutable arrays are invariant
const view: readonly Shape[] = circles; // OK: readonly views are covariant
const copies: Shape[] = [...circles];   // OK: explicit copy reifies Shape elements
```

For the other spellings of "a collection of shapes", the element representation decides everything:

| Element type | `Shape[]` means | Holds |
| --- | --- | --- |
| `class Shape` | array of managed references | any subclass, open set |
| `type` / `newtype` union | array of tagged variant layouts | the listed variants, closed set |
| `interface Shape` | induced generic, `Array<T: Shape>` | one concrete `T`, homogeneous |
| `Dynamic<Shape>` | array of erased fat pointers | any implementor, open set |

### Static

Unlike TypeScript, Destack actually _compiles_, so we need to figure out during "compile time" the final type of each value and fill in values for all the known constants.
To do this, the "evaluation time" of the program is conceptually split into three successive worlds that only flow forward:

| World | Meaning | Example |
|-------|---------|---------|
| Static | types, values, and relations known while checking | `T`, `N`, `this.Width`, `T extends string? A : B` |
| Comptime | ordinary code explicitly evaluated by the compiler | `comptime factorial(10)` |
| Runtime | ordinary program execution | `readFile(path)`, `worker.postMessage(msg)` |

The statically known language forms known to inference are called **static terms**: static evaluation happens automatically during inference, it is restricted to a small subset of the language (like TypeScript type operators), and it can _not_ execute `comptime <expr>` expressions.
That keeps compilation fast and predictable, and thanks to TypeScript's flexible type algebra, static terms are still pretty powerful:

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

Static terms are required wherever the language needs an inference-known answer: fixed array lengths, conditional types, associated members, static decorators, [layout queries](#layout), and [placement algebra](#algebra).
Type inference may flow _out_ of modules, but Destack does not support circular static inference or inference across modules in any way.

```ds
type Block<comptime N: uint> = [uint8; N];
type Payload<T> = T extends string ? Utf8Payload : BinaryPayload;

struct Buffer<T, comptime Mode: "inline" | "external"> {
    @if(Mode == "inline")
    index: InlineIndex;

    @if(Mode == "external")
    index: ExternalIndex;

    data: T[];
}
```

In the `Buffer` example, `Mode` is carried as a generic value until `Buffer<T, "inline">` or `Buffer<T, "external">` is instantiated.
At that point the [`@if`](#static-if) guards become ordinary yes/no decisions and the concrete shape is known.
The same idea applies to guarded statements: while a generic declaration is still open, a guarded statement is checked under its guard, and when the declaration is instantiated the statement is either present or gone.

```ds
function size<comptime Wide: boolean>(): Wide extends true ? 8 : 4 {
    @if(Wide)
    return 8;

    @if(!Wide)
    return 4;
}
```

### Associated Types and Constants

Associated types and constants contribute static members to a type that can be reused within the type and its implementors but do not need to be exposed to every single caller.
Both associated types and constants also work in abstract types, and as they are associated with the type directly, they do not occupy any instance space on the type.
All statically known types and constants share the same static evaluation logic, and thus associated types and constants also mix with generic parameters, conditional types, decorators, and so on.

```ds
newtype interface Iterator {
    type Item;

    next(): Option<this.Item>;
}

function collect<I: Iterator>(iter: I): I.Item[] {}
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
declare function read<I: Iterator<type Item = uint8>>(iter: I): Option<uint8>;
declare function readBlock<T: RegisterBlock<comptime Width = 16>>(block: T): [uint8; 16];
```

As a nice bit of "sugar", positional arguments can also be used to refine associated members in declaration order - just like all other generic parameters so `Iterator<uint8>` resolves to `Iterator<type Item = uint8>`.

### Constraints

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

### Shapes

Because Destack inherits TypeScript's type forms of `class`, `type` and `interface`, and then _adds_ `struct` value types and nominality via `newtype`, we now have a spectrum of _six_ different ways of spelling that something looks like a `Point { x: number; y: number }`.
Bleh. 
It is what it is.
A little unfortunate, but we couldn't figure out a good way to compress these shapes without losing either key additions like nominality and value types or compatibility guarantees like `type`, `interface`, and `class`.
So, here goes:

| Form | Role | Representation |
| --- | --- | --- |
| `type Point = { x: number; y: number }` | Transparent alias for a type expression. | Transparent |
| `interface Point { x: number; y: number }` | Named structural constraint with extension syntax. | Transparent |
| `newtype interface Point { x: number; y: number }` | Nominal interface (trait). | Transparent |
| `newtype Point = { x: number; y: number }` | Nominal wrapper over an (object) shape. | Managed object |
| `struct Point { x: number; y: number }` | Nominal value product. | Owned value |
| `class Point { x: number; y: number }` | Nominal identity object. | Managed object |

Hopefully, these mostly behave as expected, even if the assortment is bigger than what one would usually get.
They do all actually fill slightly different niches, and, fortunately, they compose quite well, and let us think in terms of types, transparency, nominality, and layout as needed, which is quite neat.

### Representation

Destack's general philosophy is to let users opt _in_ to additional control and complexity as needed - as much as possible, things should "just work" like in TypeScript.
That said, the actual memory representation of types does matter, and our unique blend of TypeScript type algebra and actual systems-y AOT compilation means that Destack needs to make representation tradeoffs differently from other languages.
Specifically, we distinguish three main axes of type behavior that affect representation:

| Axis | Question | Examples |
| --- | --- | --- |
| *nominal* vs *structural* | must the type be constructed or implemented explicitly? | `newtype` / `struct` / `class` / `newtype interface` vs `type` / `{ x: number }` |
| *transparent* vs *opaque* | does the name just expand to a type expression? | `type` vs `interface` / `struct` / `class` / `newtype` |
| *concrete* vs *abstract* | is there already a representation we can store directly (the builtin `Concrete` bound)? | primitives / `struct` / `class` vs bare constraints |

The position decides who commits to a representation: a _constraint_ position leaves the choice to each use site, while a _storage_ position makes the declaration commit.
All storage positions induce an implicit `Concrete` requirement on the value being stored, and `Concrete` is also available explicitly for the few cases where that's helpful (e.g. `type InlineBytes<T: Concrete> = [uint8; sizeOf<T>()]`).
Wherever a type can have multiple possible runtime representations, Destack induces an _implicit_ generic parameter and monomorphizes all applications of that parameter, just like with an explicit generic type:

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
    position: { x: 1, y: 2, z: 3 } // ERROR: stored `Point` has exact layout
};
```

Transparent types - type aliases - induce a generic in constraint positions (like a function parameter) but are encoded directly in storage positions (like a field in an aggregate).
This is primarily such that the common pattern of `class Player { status: "running" | "walking" | "idle" }` (which is really just a type alias) works as expected without any additional confusing generics.
Unlike transparent type aliases, interfaces also induce a generic in storage positions (even as both behave like structural types in general):

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

Desugared, that is roughly:

```ds
struct Rectangle<TStart: PointLike, TEnd: PointLike> {
    start: TStart;
    end: TEnd;
}
```

Anonymous constraint expressions require a specific type to be filled in at usage sites and thus induce a generic:

```ds
newtype interface Writer {
    write(bytes: [uint8]): Result<uint, Error>;
}

interface Writer {
    write(bytes: [uint8]): Result<uint, Error>;
}

type Writer = {
    write(bytes: [uint8]): Result<uint, Error>;
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

### Dynamic

The default being that structural constraints become hidden generic parameters is _generally_ great for performance in a `type`-heavy language like TypeScript, and it works especially well because we always compile statically from source.
However, sometimes explicit _runtime_ indirection / erasure is desired, and Destack also provides an intrinsic `Dynamic<T>` wrapper as the explicit erased runtime value satisfying any dynamic-safe `T`:

```ds
// just like the function, this Logger is implicitly generic over Writer
struct Logger {
    writer: Writer;
}

// the Logger above is the same as Logger<T: Writer>
struct Logger<T: Writer> {
    writer: T;
}

// for fixed layout, erase the constraint
struct LoggerFor {
    writer: Dynamic<Writer>;
}
```

For a type `T` to become concrete (as required by `Dynamic<T>`), it must have a surface we can actually erase into a runtime witness: we call this `DynamicSafe`.
Basically, the `T` in `Dynamic<T>` implies `DynamicSafe`, which is very similar to Rust's "object-safe" requirements for `dyn T`:
 - no generic members that introduce new generic parameters
 - no index signatures
 - no unqualified reference to `this`

Of course, the `Dynamic<T>` value _itself_ is always `Concrete`: erasure is precisely what gives an abstract constraint one fixed sized runtime representation.

### Layout

Layout is the concrete storage and ABI shape selected for a representable type under the active target, the default representation being `@repr("destack")`.
Only represented types have layout; structural shapes become represented when a representation slot reifies them, while incomplete constraints must be preserved through a generic parameter or erased behind `Dynamic<T>`.
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

### Reflection

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

## Expressions

### Values

Destack supports "expressions as values" where (almost) all statements are expressions that produce values, and the last expression (no trailing `;`) becomes the value of the overall expression.

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

If-let expressions enable nice sugar for matching a value with a refutable pattern in a conditional.
Bindings from the pattern are available in the positive branch:

```ds
const result = if (let value! = maybe) {
    value
} else {
    0
};

if (let (x, y) = point) {
    print(x + y);
}
```

`do { ... }` turns a block into an expression when braces would otherwise be ambiguous with an object expression or statement block.
The final expression without a trailing semicolon becomes the block value.

```ds
const user = do {
    const record = loadUser(id)?;
    User.fromRecord(record)
};
```

### Closures

Closures in Destack work essentially like TypeScript's closures, capturing the surrounding lexical environment and preserving lexical `this` around a generic `Function<Parameters, Return>`.
"Arrow function types" are syntax sugar for that form, so `(message: string) => Result<void, IOError>` is the same type as `Function<(string,), Result<void, IOError>>`.

```ds
let count = 0;

const next = () => {
    count += 1;
    return count;
};
next satisfies () => number;
next satisfies Function<(), number>;

const read = () => count;
```

As with all of Destack, the `Function`s behind closures behave like one would expect in TypeScript by default, with additional control available on demand via the regular memory modifiers like `&Function<(string,), void>`.
By default, captures preserve variable identity: the captured variable's storage is automatically managed by the compiler, and every closure that captures that variable observes the same storage.
The capture policy can be configured via the `@capture` decorator:

| Policy | Meaning |
| --- | --- |
| `"manage"` | preserve variable identity through compiler-managed storage |
| `"borrow"` | capture borrowed access to the original binding |
| `"copy"` | snapshot the current value |
| `"move"` | move the binding into the closure |

Explicit capture forms choose how the callable value itself will be stored in the closure environment:

```ds
let count = 0;

@capture("borrow")
let borrowed: &Function<(), int32> = () => count;

let value = 0;

@capture("move")
let owned: ^Function<(), int32> = () => value;

let state = 0;

let managed: Function<(), int32> = () => state;
```

The short form for `@capture` sets the default for every captured binding, and the object form overrides selected bindings, including `this`:

```ds
class Client {
    prefix: string;

    make(socket: Socket, logger: Logger): (message: string) => Result<void, IOError> {
        @capture({
            default: "copy",
            socket: "move",
            logger: "borrow",
            this: "borrow",
        })
        return (message) => {
            logger.info("sending");
            return socket.write(`${this.prefix}: ${message}`);
        };
    }
}
```

Of course, closures with custom capture behavior must still follow general ownership rules - for example, if one closure moves a binding, later uses or captures of that binding are rejected.

### Continuations

Async functions and generators are closures that can pause and be resumed later via stackful `Continuation`s: the runtime parks the live frame and hands back a `ContinuationHandle` - an ordinary owned value whose one-shot `resume(value)` continues the frame and whose `Drop` unwinds it, under the usual ownership rules.
`Promise`, `Generator`, and `AsyncGenerator` are "just" standard library types that store such handles in ordinary fields:

| Form | Meaning |
|------|---------|
| `ContinuationHandle` | one-shot owned handle to a parked frame, `Drop` unwinds it |
| `Promise<T>` | Worker-local async result object |
| `Generator<Y, R, N>` | Worker-local suspended generator |
| `AsyncGenerator<Y, R, N>` | Worker-local suspended async generator |
| produced `T` | value eventually produced by async code |

As in TypeScript, `await` and `yield` are (stackful) suspension points: the entire stack up to that point is parked, and some other task gets to run.
In pure managed land, suspension works as before, and managed values can be stored in parked frames because it's all - well - managed.

```ds
type User = { name: string; };

async function read(user: User): Promise<string> {
    const name = user.name;
    await tick();
    return name; // valid as before
}
```

### Tasks

Stackful continuations and (relatively) cheap Workers make Destack's concurrency ergonomic and _structured_ by default, while also keeping TypeScript's familiar `Promise` behavior: calling an async function starts it eagerly and returns a worker-local `Promise<T>` that can be stored, combined, and awaited as usual.
There is no special machinery required for this - pending handles live in ordinary places (like a `Promise`'s reactions, the scheduler's queue, a `Task`), so structure is just storage and cancellation is just `Drop`.
Nice.
Work that outlives its frame can be spawned into an explicit `TaskScope`, which cannot exit until its children complete or are cancelled (the structured concurrency model of Trio and Kotlin):

```ds
async function crawl(seeds: [Url]): Promise<Report> {
    await using scope = TaskScope.open();

    const pages = seeds.map((seed) => scope.spawn(() => fetch(seed)));
    const results = await Promise.all(pages);

    return Report.from(results);
} // the scope cannot exit while children are pending; pending children are cancelled on unwind
```

Cancelling a task resumes its parked continuation into the unwind path at its suspension point, cleanup (`using` / `finally` / `Drop`) runs deterministically, and the unwind stops at the task boundary - the Worker carries on, and `Cancelled` surfaces only at `join()`.
By the time a frame exits normally, everything it started must be awaited, returned, or handed to a scope - the `no-floating-promises` rule, `deny` by default in `.ds` (strict TypeScript codebases already lint this, Destack just means it) - and when a frame unwinds instead, its still-pending children are cancelled:

```ds
async function refresh(cache: Cache): Promise<void> {
    fetchAndStore(cache); // ERROR: floating promise, await it or hand it to a scope
}
```

Because a scope guarantees a join before the parent frame dies, borrows of owned or static data _may also cross into scoped child tasks_ (subject to `Send`) - fork-join parallelism over borrowed data without ceremony (!).
And because the runtime owns continuation scheduling, our DST mode can intercept every suspension point and replay or explore schedules deterministically.

### Patterns

TypeScript has pattern based destructuring for arguments and assignment-like expressions, and Destack extends that idea into `match`, `if (let ...)`, `let ... else`, and `catch match` with a full suite of patterns for every type family:

| Family | Example | Meaning |
|--------|---------|---------|
| Wildcard | `_` | match and ignore the value |
| Binding | `value` | bind the matched value |
| Literal | `"ok"`, `0`, `true` | match one literal value |
| Range | `0..10`, `..=255` | match an integer, `bigint`, or `char` interval |
| Tuple | `(x, y)` | destructure a tuple value |
| Array, slice, fixed array | `[head, ...tail]` | destructure indexed elements |
| Object | `{ kind: "ok", value }` | destructure a structural object |
| Nominal object | `Point { x, y }`, `User { name }` | match a nominal object-shaped value and destructure stored fields |
| Newtype | `UserId(value)`, `Config({ debug })` | unwrap a nominal newtype |
| Enum | `State.Ready` | match a nominal enum variant |
| Union | `0 | 1 | 2` | accept any listed pattern |
| Rest | `...tail` | collect the remaining elements or fields |
| Default | `name = "guest"` | bind a fallback for missing destructured values |
| Must | `value!` | bind the non-nullish value |
| Borrow binding | `&readonly value`, `&value`, `&exclusive value` | bind the selected place through a borrow |
| Move binding | `^value` | bind the selected place by ownership |
| Dereference | `*Point { x, y }` | dereference the selected place before matching |
| Guard | `pattern if (condition)` | require an extra boolean condition |

For exhaustive matching with those patterns, Destack supports the `match` expression:

```ds
match (result /* Result<T, E> */) {
    Ok { value } => process(value)
    Err { error } if (isRetryable(error)) => retry()
    Err { error } => fail(error)
}
```

Like with other conditional expressions, the resulting type of a match expression is the union of its arms' types.

```ds
declare const point: Point;
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at ${x}, ${y}` // irrefutable if point: Point
}
```

Some patterns are irrefutable, meaning they always match, and then no fallback branches are needed at all.
Refutable patterns require some fallback such that all branches are always covered: a `match` fallback arm, an `else` branch for `if (let ...)`, or an `else` continuation for `let ... else`.

```ds
declare const point: Point;
match (point) {
    Point { x: 0, y } => "vertical"
    Point { x, y: 0 } => "horizontal"
    _ => "neither" // required fallback
}

declare const maybe: Option<int32>;
let value! = maybe else {
    return Result.err("missing value");
};
```

It should be noted that unlike with construction (`{ ... }` for objects, `T { ... }` for structs, `new T(...)` for classes), the pattern destructuring unifies structs and classes into a single nominal object pattern (`T { ... }`).
Admittedly, this is a little suboptimal since it's not perfectly symmetrical, but we couldn't think of a more reasonable syntax that's not ambiguous or "magic" in some worse way.

```ds
class User {
    name: string = "";

    get displayName(): string {
        return this.name;
    }
}

declare const user: User;

match (user) {
    User { name } => name
}
```

The patterns match only real fields, not getters or setters:

```ds
match (user) {
    User { displayName } => displayName // ERROR: getter
}
```

### Guards

Guards are boolean expressions that can refine types, like `"name" in value`, `instanceof`, and `value is T` checks:
 - `"name" in value` for object types, and it is quite imprecise.
 - `instanceof` for classes.
 - `value is T` for primitive, nominal, and class cases.

Destack does not need (or support) the vague `typeof` check, but supports an additional precise `value is T` to check whether the current runtime representation of `value` carries the case or identity for `T`:

```ds
struct User {
    name: string;
}

function label(value: User | string): string {
    if (value is User) {
        return value.name;
    } else {
        return value;
    }
}
```

Like other guards, `value is T` returns `boolean` and narrows the branch:
 - When `true`: narrows to the part of its current type that can be `T`.
 - When `false`: narrows away the covered part when that can be represented.

For union values, the test checks the union representation:

```ds
const value: string | int32 = 1;

if (value is string) {
    value satisfies string;
} else {
    value satisfies int32;
}
```

### Loops

For convenience and clarity, Destack supports `loop` as the explicit infinite loop form, and like other expressions, loops can produce a value through `break`.

```ds
const line = loop {
    const input = readInput();
    if (input == "quit") {
        break "done";
    }
    process(input);
};

let status = outer: loop {
    break outer: "done";
};
status satisfies "done";
```

The `break` operand still follows TypeScript's label rule, of course: a lone identifier is still a label.

### Using

Resource management with `using` and `await using` follows the [TC39 explicit resource management proposal](https://github.com/tc39/proposal-explicit-resource-management), but of course with nominal interfaces instead of magic `Symbol` keys:
- `using` accepts `Dispose | null | undefined`.
- `await using` accepts `AsyncDispose | Dispose | null | undefined`, and falls back to synchronous disposal when the resource only implements `Dispose`.
- `null` and `undefined` are ignored, following the spec.

Resources are cleaned up at lexical scope exit in LIFO order, and `await using` runs async cleanup when required.
Cleanup - that is, the dispose function - runs when the scope exits for any reason: fallthrough, `return`, `break`, `continue`, or `?`.
The same using form also works in loop form, where it applies for every iteration.

```ds
{
    using input = openFile(inputPath),
          output = openFile(outputPath);

    copy(input, output);
} // output is disposed, then input is disposed

async function runQuery(sql: string): Result<Row[], DatabaseError> {
    await using connection = await pool.connect();

    return await connection.query(sql);
} // connection is disposed and awaited

for (using file of files) {
    process(file);
} // file is disposed after each iteration
```

### Operators

Destack extends TypeScript operators with typed overloads and some additional precision.
Logical operators (`&&`, `||`, `??`), optional chaining, assignment, and strict identity (`===`, `!==`) are not (directly) overloadable, as usual, and same for increment (`++`) and decrement (`--`).
Compound assignment operators like `+=` are desugared into their component operations (`+` and `=`), and are thus indirectly overloadable.

| Operator | Example | Interface |
|----------|---------|----------|
| `+` | `a + b` | `Add<T>` |
| `-` | `a - b` | `Subtract<T>` |
| `*` | `a * b` | `Multiply<T>` |
| `/` | `a / b` | `Divide<T>` |
| `%` | `a % b` | `Remainder<T>` |
| `**` | `a ** b` | `Power<T>` |
| `+` | `+a` | `Plus` |
| `-` | `-a` | `Negate` |
| `&` | `a & b` | `And<T>` |
| `\|` | `a \| b` | `Or<T>` |
| `^` | `a ^ b` | `Xor<T>` |
| `~` | `~a` | `Not` |
| `<<` | `a << b` | `ShiftLeft<T>` |
| `>>` | `a >> b` | `ShiftRight<T>` |
| `>>>` | `a >>> b` | `ShiftRightUnsigned<T>` |
| `==`, `!=` | `a == b` | `PartialEqual<T>` |
| `<`, `<=`, `>`, `>=` | `a < b` | `Compare<T>` or `PartialCompare<T>` |
| `[]` | `a[i]` | `Index<I>` |
| `[] =` | `a[i] = v` | `IndexSet<I, V>` |
| `*` | `*a` | `Dereference<"readonly">` |
| `* =` | `*a = v` | `Dereference<"exclusive">` |

Equality `==` / `!=` follows Rust's split between "partial" and "total" equality:
- Equality operators `==` and `!=` dispatch through `PartialEqual<T>.equal`
- `Equal<T>` is a stronger _marker_ interface based on `PartialEqual<T>`

The distinction between `Equal` and `PartialEqual` exists mainly to deal with the oddities of floating point numbers (since e.g. `NaN` does not equal itself, definitionally).
Comparison operators `<` / `>` follow the same pattern and support `PartialCompare<T>` when ordering may be undefined (e.g., floats), and `Compare<T>` when ordering is total (e.g., integers).
Strict identity `===` still keeps its TypeScript meaning: by value for primitives (including `string` and `char` contents, and `NaN === NaN` is still `false`), and by reference identity for managed objects.

Dereference operators are a little different from the main "value-shaped" operators, because `Dereference<A>` transparently _dereferences_ (projects) access forms on use.

```ds
struct Box<T> {
    ptr: ^T;
}

extension<T> of Box<T> implements Dereference<"readonly"> {
    type Output = &T;

    dereference(): this.Output {
        &*this.ptr
    }
}
```

For example, `*box` flows through `Dereference<"readonly">`, while assignment through `*box` needs `Dereference<"exclusive">`.
Importantly, member lookup and method calls may auto-dereference transparently through `Dereference` without any special syntax - this is what enables ergonomic access to smart pointer like wrappers and guards.

### Arithmetic

Destack's sized numeric types are checked for correctness _in all modes_, no ifs or buts, no debug-only checking and no silent release-mode wraparound:

- **Overflow traps.** Arithmetic on sized integer types (`+`, `-`, `*`, `**`, `<<`, negation) traps when the result does not fit the type.
- **Division traps.** Integer division and remainder by zero trap (`/`, `%`), as does overflowing division (`int32.MIN / -1`).
- **Floats follow IEEE-754.** Float arithmetic never traps; division by zero, `NaN`, and infinities behave exactly as in TypeScript.

When modular or clamping semantics are desired, we can say so explicitly with the standard library helpers:

```ds
declare const a: uint8;
declare const b: uint8;

a.wrappingAdd(b) satisfies uint8;            // modular arithmetic
a.saturatingAdd(b) satisfies uint8;          // clamps at the bounds
a.checkedAdd(b) satisfies uint8 | undefined; // detects overflow as a value
Wrapping(a) + Wrapping(b);                   // wrapping by type
```

Conversions follow the same explicitness rule as the rest of Destack: `as` between numeric types is allowed only when every value of the source type is representable in the destination (lossless), and lossy conversions must pick their behavior explicitly:

```ds
declare const wide: int32;

const a: int64 = wide as int64;   // OK: lossless widening
const b: uint8 = wide as uint8;   // ERROR: lossy conversion
const c = wide.truncate<uint8>(); // explicit: keep the low bits
const d = wide.saturate<uint8>(); // explicit: clamp into range
const e = wide.tryInto<uint8>();  // explicit: uint8 | undefined
```

Similarly, mixed-type arithmetic (different widths or int/float mixes) requires explicit conversion to a common type first; the only implicit numeric flow is literal typing as described in [Primitives](#primitives).

### Ranges

Range expressions like `a..b` produce range values for slicing, indexing, iteration, and any APIs that want to think in terms of bounds.
Like Rust and Python, the default range is half-open, and all range forms implement `RangeBounds<T>`, whose `startBound()` and `endBound()` methods return `Bound<T>`:

| Expression | Type | Meaning |
|------------|------|---------|
| `start..end` | `Range<T>` | Include `start`, exclude `end` |
| `start..=end` | `RangeInclusive<T>` | Include both bounds |
| `start..` | `RangeFrom<T>` | Include `start`, no end bound |
| `..end` | `RangeTo<T>` | No start bound, exclude `end` |
| `..=end` | `RangeToInclusive<T>` | No start bound, include `end` |
| `..` | `RangeFull` | No start or end bound |

Ranges work in patterns and subscripts exactly like one would expect from other languages:

```ds
let values: Slice<int32> = [1, 2, 3, 4, 5];

values[1..4] satisfies Slice<int32>;
(&values)[1..4] satisfies &Slice<int32>;
(&readonly values)[1..4] satisfies &readonly Slice<int32>;
(&exclusive values)[1..4] satisfies &exclusive Slice<int32>;
```

### Dispatch

"Dispatch" is how calls, member accesses, and overloadable operators select the specific field or method to use.
Destack's overload resolution rule is based on TypeScript: build the candidate set, keep candidates compatible with the arguments as written, then pick the first match _in declaration order_:

#### Overloads

Unlike in TypeScript, there may be multiple overloaded _implementations_ for the same name and scope:

```ds
function parse(input: string): int32 {
    return parseInt(input);
}

function parse(input: int32): int32 {
    // legal, actually different implementation!
    return input;
}
```

Members work in the same way (after receiver lookup): inherent members first, then visible extension members in declaration order.
Because Destack has strict types, property access and method calls may use different access paths; that is, a field and method can share a source name: `value.name` resolves the field/accessor projection, while `value.name()` resolves the method-call projection.
(Fields and accessors share the property projection and therefore cannot share a name.)

#### Operators

For overloadable operators, the operator decides the interface to check, and the left operand is the receiver (matching how it is written in the interface implementation).
Binary operators do not fall back to the right operand, so if both operand orders are desired - `a + b` and `b + a`, both receiver implementations must exist.

```ds
newtype interface Add<T = this> {
    type Output;

    add(other: T): this.Output;
}

extension of Vector2 implements Add<Vector2> {
    type Output = Vector2;

    add(other: Vector2): this.Output {
        Vector2({ x: this.x + other.x, y: this.y + other.y })
    }
}

extension of Vector2 implements Add<float32> {
    type Output = Vector2;

    add(other: float32): this.Output {
        Vector2({ x: this.x + other, y: this.y + other })
    }
}

const moved = position + offset; // Add<Vector2>
const padded = position + 1.0;   // Add<float32>

moved satisfies Vector2;
padded satisfies Vector2;

1.0 + position; // requires Add<Vector2> on float32
```

#### Interfaces

As discussed in Types, structural interfaces keep normal TypeScript shape checking and mostly work exactly as expected: bare structural interface annotations are constraints, so `point: PointLike` behaves like an implicit `T: PointLike` parameter and is specialized for the concrete argument type.
Because structural interfaces are satisfied by shape, writing `implements` on one is only an explicit declaration-site check.
Erased interface values are spelled explicitly with `Dynamic<T>`.

```ds
interface PointLike {
    x: int32;
    y: int32;
}

struct Point implements PointLike {
    y: int32;
    x: int32;
}

function lengthSquared(point: PointLike): int32 {
    point.x * point.x + point.y * point.y
}
```

One point of difference to TypeScript is mutability through interfaces, because we need to actually compile the code into something with a fixed shape.
Interface fields are still read-write by default, but concrete fields satisfy mutable structural fields only when their types match exactly (after normal alias and newtype normalization).

```ds
interface PointLike {
    readonly x: int32 | float32;
}

struct Point {
    x: int32;
}

const point: PointLike = Point { x: 1 };
point.x satisfies int32 | float32;
```

Readonly fields only need, well, reads, so they can widen through ordinary implicit casts and nested structural views.
However, if `PointLike.x` were mutable, this conversion of `Point` to `PointLike` would be rejected because writing through `PointLike` would no longer be correct (it would have to implicitly widen, but it doesn't and cannot know that!).

#### Index Signatures

Index signatures like `{ [index: string]: string }` (as in `Record<K, V>`) still work like structural constraints for object-shaped values, and can be satisfied with both fixed object shapes and types implementing `Index` for readonly / `IndexSet` for writable shapes.
While Destack supports structural index signatures, the actual compiled shape must still be known, and so `Record`-like types _by themselves_ are not concrete (they're just constraints).

```ds
interface Bag<T> {
    readonly [key: string]: T;
}

// finite object view
const counts: Bag<int32> = { apples: 3, oranges: 2 };

function read<T>(bag: Bag<T>, key: string): T | undefined {
    bag[key]
}

read(counts, "apples") satisfies int32 | undefined;

// mutable indexed container
const dynamicCounts = new Map<string, int32>();
dynamicCounts.set("apples", 3);
dynamicCounts.has("apples") satisfies boolean;
dynamicCounts satisfies { [key: string]: int32 };
```

#### Unions

Member dispatch on union receivers is resolved per variant, and every variant must expose that member.
If all variants resolve to the same implementation, the call is static; otherwise the result type is the union of the selected return types and the compiler emits dynamic checks to dispatch on the right member at runtime.

```ds
struct TcpStream {
    write(chunk: [uint8]): Result<usize, IOError> {}
}

struct MemoryBuffer {
    write(chunk: [uint8]): Result<usize, never> {}
}

function writeAll(sink: TcpStream | MemoryBuffer, chunk: [uint8]) {
    const written = sink.write(chunk);
    written satisfies Result<usize, IOError> | Result<usize, never>;
}
```

#### Coherence

Unlike Rust, Destack has _no orphan rule_: any module may extend any nominal type and implement any nominal interface for it.
Because Destack always compiles whole programs from source, coherence can be enforced where it is actually needed instead of at every possible declaration site, which is quite nice:

| Claim | Scope | Conflict |
| --- | --- | --- |
| Extension members | lexical: same file, or explicitly imported | ambiguity error at the use site |
| `implements` declarations | global: unique per (type, interface instantiation) pair | compile error pointing at both declarations |
| Operator and capability dispatch | global: works wherever the interface is nameable | none possible, implementations are unique |

For extension members, candidates resolve in declaration order within one scope, import order is never considered, and an overload that can never win is flagged as unreachable.
For `implements`, uniqueness covers blankets too (no specialization, and bare-bounded blankets only from the interface's package, see [Blankets](#blankets)) - and it is what keeps generic instantiation coherent: a `Set<Vector2>` is always built and queried under the same `Hash` implementation, no matter which modules the value flows through.

When two packages do collide, the build fails and the fix is source-level (drop a dependency, or vendor and patch) - whichever side "won" would silently change the other's behavior, so there is deliberately no switch to pick one.
Libraries should therefore only implement pairs they own one side of (a default-`warn` diagnostic nudges accordingly); foreign-on-foreign implementations belong in applications, where nothing downstream can collide.

### Errors

The banishing of exceptions is Destack's most immediately noticeable divergence from TypeScript: Destack uses **Result-first error handling** exclusively, and throwing exceptions is not allowed in any native Destack code.
Recoverable errors use `Result<T, E>`, integrate with `try` / `catch`, and can be propagated with `?`, `??`, and force-unwrapped postfix `!`.
(JavaScript exceptions remain valid _syntax_ because we need to integrate with JS targets directly, but in regular userland, exceptions are forbidden.)

The same rule also extends to async code: a Destack `Promise<T>` never rejects, because rejection is just an asynchronous exception.
Async failure travels as `AsyncResult<T, E>`, [panics](#panics) unwind the Worker, and rejection-capable host promises are adopted into `AsyncResult` (or panic) at the host binding boundary.

#### Error

Like in Rust, types that want to be handled as general errors explicitly implement the nominal `Error` interface:

```ds
newtype interface Error {
    display(): string;

    source(): Dynamic<Error> | undefined {
        undefined
    }
}
```

#### Result

Destack provides `Result<T, E>` as the primary error handling mechanism:

```ds
export struct Ok<T> {
    kind: "Ok" = "Ok";
    value: T;
}

export struct Err<E> {
    kind: "Err" = "Err";
    error: E;
}

export newtype Result<T, E> = Ok<T> | Err<E>;
```

We typically construct results through `Result.ok(value)` and `Result.err(error)`.
The variants are ordinary nominal data, so pattern matching works directly:

```ds
declare function parseInteger(raw: string): Result<int32, ParseError>;

function parsePort(raw: string): Result<uint16, ParseError> {
    const value = parseInteger(raw);
    if (value < 0 || value > 65535) {
        return Result.err(ParseError(`port out of range: ${raw}`));
    } else {
        return Result.ok(value as uint16);
    }
}

match (parsePort(input)) {
    Ok { value } => connect(value)
    Err { error } => report(error)
}
```

`Result` is great for synchronous error handling, and `AsyncResult` extends the exact same idea to `Promise`-based asynchronous errors with a convenient wrapper around `Promise<Result<T, E>>`.

```ds
export newtype AsyncResult<T, E> = Promise<Result<T, E>>;

declare function fetchUser(id: UserId): AsyncResult<User, NetworkError>;

async function loadProfile(id: UserId): AsyncResult<Profile, NetworkError | DecodeError> {
    const user = await? fetchUser(id); // `await? expr` is sugar for `(await expr)?`
    const profile = decodeProfile(user)?;
    return Result.ok(profile);
}
```

For `await`, we also have some `await? expr` sugar for `(await expr)?`, and `await! expr` is sugar for `(await expr)!`.

#### Maybe, Must and Coalesce

`?`, postfix `!`, and `??` all unwrap the same `Try`-based absence-or-failure shape.
Opening removes outer `null` / `undefined`, opens one `Try` carrier, and removes `null` / `undefined` from the carrier's success value.
It's much simpler than it sounds:

```ds
declare const x: Result<T | null | undefined, E | null | undefined> | null | undefined;

// x?
// -> success: T
// -> failure (propagated): E | null | undefined
```

Nullish values on the failure side remain in the failure side.
The three operators differ on the failure case:

```ds
x?      // success T, failure leaves the expression
x!      // success T, failure traps
x ?? y  // success T, failure evaluates y
```

The try operator `?` keeps the success value and lets absence or failure leave the current expression in whichever way the container requires.
Inside a `try` block with `catch`, propagation transfers the failure value to the catch instead.

```ds
function readConfig(path: string): Result<Config, IOError | ParseError> {
    const text = readFile(path)?;
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

The try-coalesce operator `??` accepts the same shape locally "within" the expression with a direct fallback instead of letting it bubble up to the container as with `?`.
The result of `Result<T, E> ?? F` is the non-nullish opened success type joined with the fallback type `T | F`:

```ds
declare const defaultConfig: Config;

declare function loadConfig(): Result<Config, IOError> | null;
const a = loadConfig() ?? defaultConfig;
a satisfies Config;

declare function loadMaybeConfig(): Result<Config | null | undefined, IOError | null> | undefined;
const b = loadMaybeConfig() ?? defaultConfig;
b satisfies Config;
```

All unwrap operators unwrap exactly _one_ layer of `Try`, so nested `Try` values inside the success type also stay wrapped at the inner layer:

```ds
declare function loadNested(): Result<Result<Config, ParseError>, IOError>;

const c = loadNested() ?? defaultConfig;
c satisfies Result<Config, ParseError> | Config;
```

Postfix `!` is the "must" forced unwrap form: it opens the same outer nullish and single `Try` layer, but [traps](#panics) instead of propagating or falling back when the value is absent or failed:

```ds
const config = loadConfig()!;
config satisfies Config;
```

Regarding precedence, whitespace decides between the try operator and a ternary, which should mostly follow how we would naturally type (and format) these expressions anyway: an attached question mark is try-propagation, a detached one is a ternary condition.
This keeps the branches unambiguous in both directions: `flag ? -x : x` is a conditional, and `x? - 1` subtracts from the opened success value (so a compact ternary requires its spaces in `.ds`).

#### Try

The standard `Try` interface is the extension point behind these operators.
A carrier names its success and failure types, can branch into either case, and can rebuild itself from a success value:

```ds
struct TryContinue<T> {
    kind: "continue" = "continue";
    value: T;
}

struct TryFailure<F> {
    kind: "failure" = "failure";
    failure: F;
}

type TryBranch<T, F> = TryContinue<T> | TryFailure<F>;

newtype interface Try {
    type Value;
    type Failure;

    static fromValue(value: this.Value): this;
    branch(): TryBranch<this.Value, this.Failure>;
}
```

For `Result<T, E>`, `Ok { value }` branches to `TryContinue<T>` and `Err { error }` branches to `TryFailure<E>`.
The branch names describe the operator's control flow, not the data constructors of any one type.

#### Try-Catch-Finally

The well known `try`/`catch` forms work with explicit `Try` propagation:

```ds
declare function readConfig(path: string): Result<Config, IOError>;

try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e) { // e: IOError
    log("failed to read config:", e)
}
```

The example uses `Result`, but any type implementing `Try` behaves the same:
- `try` does not implicitly unwrap `Result` values
- Use `?` inside the block to propagate `Try` failures into the catch
- Use `??` inside the block when the failure should be handled locally with a fallback

For convenience, Destack introduces a new `catch match` form that can branch on `Try` failures directly for some pretty pleasant syntactic sugar:

```ds
try {
    readConfig()?; // -> Result<void, MissingError>
    parseConfig()?; // -> Result<void, FormatError>
} catch match (failure) { // failure: MissingError | FormatError
    MissingError { path } => Report.wrap(failure, `missing config: ${path}`)
    FormatError { line } => Report.wrap(failure, `bad format on line ${line}`)
}
```

Finally arms run as usual after the `try` / `catch` body, including when `?` leaves the block early.

#### Panics

Unrecoverable failures are hard **panics**: panics occur when a must unwrap (`!`) fails, when an explicit `panic("...")` runs, when a runtime check traps (arithmetic overflow, out-of-bounds indexing, lone-surrogate indexing), or when `unreachable` code is reached.
There is no userland `catch` for panics, that's what makes them panics: a panic means the program is outside its specified envelope.

Mechanically, a panic unwinds the _current_ `Worker`:
1. The panic starts unwinding from the trapping point with a message payload.
2. Cleanup runs on the way out - `using` / `await using` disposal, `finally` arms, and `Drop` glue for owned values - in the usual LIFO order.
3. The Worker terminates; a parent or supervisor observes the termination and gets the `Panic` struct - message, source location, and stack trace when available - through the regular `Worker` API and decides what to do (restart, propagate, report).
4. A panic _during_ that cleanup aborts: there is no unwinding the unwinding.

Conveniently, the Worker thus also becomes the fault boundary, mirroring both the web's worker model and (roughly) Erlang-style supervision: a panic never silently corrupts sibling Workers, and the test harness and the simulator can observe panics as ordinary (deterministic) Worker terminations without any language-level catch.
Nice.
For cases where we do want to unambiguously kill the whole program, Destack supports a stronger `abort` for genuinely unrecoverable states like detected memory corruption:

| Form | Cleanup | Boundary |
| --- | --- | --- |
| `panic` | unwinds with `using` / `finally` / `Drop` cleanup | terminates the Worker |
| `abort` | none, stops immediately | terminates the whole process |

### Trees (TSX)

TypeScript XML (`.tsx`) is a great way of writing UI-shaped code and has even seen some successful adoption for other tree-shaped data structures as well.
It's not perfect, but it is very useful in many situations, and Destack (`.ds`) natively supports `.tsx`-like constructs with the same rules:

```ds
// Wall.ds
<Wall id={1}>
    <Block name="foo" color={Color.RED} />
    <Block name="bar" color={Color.BLUE} />
</Wall>;

// Prompt.ds
<Prompt>
    <System>You are a helpful assistant.</System>
    <User>{userMessage}</User>
</Prompt>;

// Level.ds
<Level difficulty={3}>
    <Player position={spawn} />
    {enemies.map(e => <Enemy {...e} />)}
</Level>;
```

Unlike in TypeScript, Destack types can participate in custom tree tag behavior by implementing the `TreeTag` interface, and custom intrinsic tags (lowercase tags like `<div>`) are created via `TreeTagBuilder`.
Essentially, `TreeTag` generalizes `jsxFactory` and `TreeTagBuilder` generalizes `jsxFragmentFactory`:
 - Uppercase or qualified tags resolve as value tags through normal value lookup and the `TreeTag` interface.
 - Lowercase unqualified tags resolve as intrinsic tags through the active `TreeTagBuilder`.

The active `TreeTagBuilder` comes from the compiler / target / profile options by default, but can be locally overridden with [module metadata](#module).

```ds
import { HtmlTree } from "destack:ui/html";

module {
    const tree = HtmlTree;
}

import.meta.tree satisfies TreeTagBuilder;
```

### Decorators

Like TypeScript, Destack uses `@` for decorator-like constructs, but Destack supports both "annotations" and "decorators", and also many more constructs can be annotated / decorated.
The syntax for both data annotations and behavior decorators is unified, the target - the thing pointed to in `@<expr>` - decides:
 - **Annotations** are _values_ like `newtype`s. They add typed metadata to the target, but don't directly change the target's behavior.
 - **Decorators** are _logic_ following some protocol that contribute code or change the analyzed shape in some bounded way.

Annotations are "inert" by default, that is, they don't do anything until either some userland construct or the toolchain give them special meaning or implement the `Macro` protocol.

```ds
newtype deprecated = () | (string,);

@deprecated("use newAPI instead") // metadata annotation, doesn't do anything
function oldAPI() {
    // ...
}

@tracked
@derive(Clone, Debug)
struct User {
    id: UserId;
    name: string;
}
```

#### Diagnostics

Like in other languages, (some of) Destack's diagnostics can be tuned with scoped decorators:
 - `@allow`: explicitly allow a specific diagnostic
 - `@warn`: warn about a specific diagnostic
 - `@deny`: error about a specific diagnostic
 - `@forbid`: forbid a specific diagnostic (cannot be overridden by `@allow`)
 - `@expect`: expect a specific diagnostic (suppress, error if not produced)

```ds
@allow("no-floating-promises", {
    if: import.meta.dev,
    otherwise: "deny",
    reason: "debug telemetry",
})
module {}
```

#### Restrictions

Relatedly, restrictions may be used to allow or disallow more fundamental reaching language behavior in certain scopes:

```ds
@noHeap
@noUnsafe
@noAliasingMutableBorrows
module {}
```

#### Taint

Destack systematizes the idea of "taints", "source", and "unsafe" modifiers on expressions and declarations using its taint system:
 - `@taint("tag")` marks a value as carrying some domain, `@untaint("tag")` unmarks it as no longer carrying that domain.
 - `@source("domain")` marks an operation that produces some domain, `@sink("domain")` marks an operation that receives some domain.
 - `@unsafe` marks an operation that is unsafe to call, `@safe` marks an operation that is safe to call.

```ds
@unsafe
declare function read<T>(pointer: *T): ^T;

@safe
function get<T>(items: Slice<T>, index: usize): T {
    if (index >= items.length) {
        panic("index out of bounds");
    }

    return items.unsafeGet(index);
}
```

#### Derive

Similar to Rust, Destack supports `@derive` providers for extending annotated declarations at compile time during the macro expansion phase.
Unlike in Rust, a derive provider is just a nominal decorator that happens to implement the `Macro<Target>` interface, and `derive`-like macros do not need to be implemented in a different package.

```ds
@derive(Clone, Debug)
struct User {
    id: UserId;
    name: string;
}

@derive(Tagged({ case: "UpperCamelCase" }))
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };
```

Destack supports all the common capability-like derives one would expect from a systems-y language, with the notable addition of `Serialize`, `Deserialize`, and `Tagged`:

| Derive | Library identity | Applies to | Explicit failure |
|--------|------------------|------------|------------------|
| `Copy` | `destack:memory.Copy` | nominal value types whose fields are all copyable | field or representation is not copyable |
| `Clone` | `destack:memory.Clone` | nominal value types whose fields are cloneable | field is not cloneable |
| `Default` | `destack:memory.Default` | nominal value types whose fields have defaults | field has no default |
| `Debug` | `destack:ops.Debug` | nominal value types | field is not debug-formatable |
| `PartialEqual` | `destack:ops.PartialEqual` | nominal value types | field is not partially comparable for equality |
| `Equal` | `destack:ops.Equal` | nominal value types | field does not have total equality |
| `PartialCompare` | `destack:ops.PartialCompare` | nominal value types | field is not partially orderable |
| `Compare` | `destack:ops.Compare` | nominal value types | field is not totally orderable |
| `Hash` | `destack:ops.Hash` | nominal value types | field is not hashable |
| `Serialize` | `destack:serde.Serialize` | nominal value types | field cannot be serialized by the selected serializer |
| `Deserialize` | `destack:serde.Deserialize` | nominal value types | field cannot be deserialized by the selected deserializer |
| `Tagged` | `destack:decorator.Tagged` | discriminated newtype unions | declaration is not a supported tagged union |

Also unlike Rust, Destack's `derive` supports automatic globally configured (and module/target/..-overridable) derives that are applied by default without explicit `derive` annotation whenever possible.
This is very convenient since most types do in fact want all the same basic well known `derive`s, but we can also trivially disable this globally, or override it per-item with an empty `@derive()`, _or_ at the module level with `module { derive: [] }`.

#### Static If

Destack also supports a special intrinsic `@if` decorator that gates the inclusion of certain nodes based on a static term.
When the condition is false, the annotated item is (in effect) removed from the instantiated shape.

```ds
interface FileSystem<Mode: "fast" | "slow" = "fast"> {
    open(path: string): Result<File, IOError>;

    @if(import.meta.platform != "windows")
    chmod(path: string, mode: uint16): Result<void, IOError>;

    @if(import.meta.platform == "windows" && Mode == "fast")
    setAttributes(path: string, attrs: WindowsFileAttributes): Result<void, IOError>;
}
```

Static ifs may annotate any meaningfully _removable_ source contribution - if removing the annotated node would leave the parent with a coherent shape, we can guard it:

| Context | Nodes | Static inputs |
| --- | --- | --- |
| Module level | imports, re-exports, top-level declarations, `module { ... }` entries | profile, module metadata, literals |
| Declaration members | struct fields, class / interface / extension members, enum variants | containing declaration generics and static members |
| Expression positions | statements, match cases, call / tree / generic arguments, tuple elements, object and type literal fields | enclosing static and generic context |

### Module

Destack modules can contain (up to) one static `module { ... }` declaration block for source-level configuration that needs to be specific to a module.
Usually, we would configure via the compiler / target / profile options, but sometimes it's helpful to refine a specific module locally:

```ds
import { HtmlTree } from "destack:ui/html";

@noHeap
module {
    const tree = HtmlTree;
    const derive = ["Debug", "Clone"];
    const product = "editor";
}
```

Decorators on `module` apply rules to the _whole_ source module, and declarations inside `module` define readonly module metadata available through `import.meta`:

```ds
module {
    const role = "server";
    const labels = {
        feature: ["search", "billing"],
    };
}

import.meta.role satisfies "server";
import.meta.labels.feature satisfies readonly ["search", "billing"];
```

It should be noted that module declarations (like global declarations) are proper static constants that are evaluated as static terms during compile time, so we could also do something slightly more dynamic like `const role = import.meta.test ? "server" : "client";`

### Globals

TypeScript supports ambient global typings, which were designed for typing the "magic" global objects provided by embedders, but it has no way to contribute _value_ globals in userland.
Destack supports "real" value `global { ... }` declarations to define such globals that can then be automatically included everywhere by (explicit) reference in the compiler / target configuration.
The active set of modules to consider for `global` declarations is configured via the `globals` field in the compiler / target configuration.

```ds
// browser-globals.ds
global { // just omit the `declare`!
    const window: Window = runtime.browser.window();
    const document: Document = runtime.browser.document();
}
```

A global block may also re-export named bindings from another module into the ambient globals.
Indeed, that is the same mechanism the well known Destack prelude uses to inject intrinsic language items:

```ds
global {
    export { Add, Subtract } from "destack:ops";
}
```

### Comptime

Inspired by modern languages like Zig and Jai, Destack supports compile-time evaluation with `comptime` expressions: ordinary code to be evaluated by the compiler, during compile time, and the results baked into the emitted artifact.

```ds
const LOOKUP_TABLE: uint8[] = comptime {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

Functions do not need to declare themselves as either "comptime" or "runtime": the same function can run at compile time when all inputs are static, and at runtime when some input is only known at runtime:

```ds
function factorial(n: int): int {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

const COMPTIME_CONST = comptime factorial(10);    // compile time
COMPTIME_CONST satisfies int;

const RUNTIME_CONST = factorial(getUserInput()); // runtime (in this case, at module initialization time)
RUNTIME_CONST satisfies int;
```

When runtime execution would be meaningless or unsafe, a function can be declared `comptime function` to declare that a function has no runtime callable form, but otherwise uses normal function syntax.
(This is in some way the opposite of the usual `constexpr` based keyword, that is, Destack functions are evaluated at `comptime` by usage, and can be marked `comptime` to force compile-time evaluation.)

```ds
comptime function fieldOffset<T>(name: string): usize {
    // inspect `T` at compile time
}

const offset = comptime fieldOffset<User>("name");
```

The evaluation scope for each comptime expression is isolated to its declaration site, and the only way to get a value "out" is to use comptime as an expression - no reaching into statics or globals allowed.
(Locals _inside_ the expression may of course be mutated.)

```ds
const WIDTH = comptime {
    let width = 4;
    width *= 2;
    width
};

let counter = 0;
comptime {
    counter += 1; // ERROR: outer mutation
}
```

Comptime blocks can also appear as members on object-like types for static checks, where they run in the static environment of the declaration or instantiation that they appear in post-inference, and they can access the same [static terms](#static) as `@if`.

```ds
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0 && size <= 65536);
    }

    data: [uint8; size];
}
```

Because comptime expressions are late-evaluated expressions, comptime conditions type check like ordinary conditions.
Both branches are analyzed, and the expression type is still the joined branch type.
The compiler may eliminate the untaken branch before final lowering when the condition is computed from static inputs:

```ds
function isPowerOfTwo(value: uint): boolean {
    if (value == 0) {
        return false;
    }

    let n = value;
    while (n > 1) {
        if (n % 2 != 0) {
            return false;
        }
        n /= 2;
    }

    return true;
}

function blockCost<comptime Width: uint>(): int32 {
    if (comptime isPowerOfTwo(Width)) {
        return 1;
    } else {
        return 2;
    }
}

function blockMultiply<comptime Width: uint>(a: int32, b: int32): int32 {
    comptime {
        assert(isPowerOfTwo(Width));
    }
}
```

Of course, comptime results must also be lowerable into the target artifact.
Plain data such as numbers, strings, arrays, tuples, objects, structs, and enums are all fine, but dynamic runtime resources like pointers and handles and such don't work because we can't meaningfully serialize them.

#### Dynamic Code

Generating and evaluating arbitrary code is supported via `eval` at _compile-time_ by passing a string computed at comptime:

```ds
import * as dir from "destack:reflect/dir";

const source = comptime renderParser(grammar);
const parser = comptime eval<dir.FunctionDeclaration>(source);
```

The source passed to `eval` must itself be available to comptime evaluation.
Generated code is parsed and typechecked as `.ds`, attached to the same module graph as a virtual source file, and tracked for diagnostics and artifact caching.

### Macros

Destack is statically typed and compiled, but supports macros as decorators backed by `comptime` execution and a bounded module-editing context.
As an example, consider a `memoize` decorator that turns a function into a cached ("memoized") version of itself that stores results in a cache to avoid recomputation on equal arguments.

```ds
newtype memoize = {
    capacity?: uint;
};

@memoize({ capacity: 1024 })
function load(id: UserId): Result<User, Error> {
}
```

As explained in [Decorators](#decorators), `memoize` by itself is just an inert annotation and it only receives behavior by implementing `Macro`.
The `Macro` system is based on four rules:
 1. Macro expansion is recursive and runs until there is nothing more to expand (or we encounter an error).
 2. Macros run in two phases during compilation: `expand` may contribute new symbols before final inference, while `materialize` fills in implementation details with full type information.
 3. Macros interact with their containing module through phase-specific context methods (`resolve`, `ensureImport`, `add`, `ensureDeclaration`, `addChild`, `replaceTarget`, `renameTarget`, `removeTarget`).
 4. Macro invocations are exclusively triggered by decorators implementing `Macro`; compiler-owned derives are a separate expansion path.

| Operation | Example | Meaning |
|-----------|---------|---------|
| `resolve` | `context.resolve("ROUTES")` | resolve a visible symbol in the current scope |
| `ensureImport` | `context.ensureImport("destack:collections", "Map")` | ensure an import used by generated code |
| `add` | `context.add(declaration)` | add a generated declaration to the current scope |
| `ensureDeclaration` | `context.ensureDeclaration("RouteDefinition", () => declaration)` | ensure a generated helper declaration exists |
| `addChild` | `context.addChild(member)` | add a generated child to the target declaration |
| `replaceTarget` | `context.replaceTarget(declaration)` | redirect the target symbol to a generated declaration |
| `renameTarget` | `context.renameTarget(name)` | keep the target declaration but change its visible name |
| `removeTarget` | `context.removeTarget()` | remove the target symbol from the visible declaration set |

Most basic wrapper-shaped decorators are just `rename` plus `add`.

```ds
type MemoizeState = {
    innerName: string;
    capacity: uint;
};

extension of memoize implements Macro<FunctionDeclaration, MemoizeState>
{
    static expand(
        target: FunctionDeclaration,
        context: ExpansionContext,
        config: this,
    ): MemoizeState {
        const innerName = `${context.name}Inner`;
        const wrapper = comptime eval<Declaration>(ds`
            function ${context.name}(id: UserId): Result<User, Error> {
                // placeholder
            }
        `);

        context.renameTarget(innerName);
        context.add(wrapper);

        return {
            innerName,
            capacity: config.capacity ?? 256,
        };
    }

    static materialize(
        target: FunctionDeclaration,
        context: MaterializationContext,
        config: this,
        state: MemoizeState,
    ): void {
        const implementation = comptime eval<Declaration>(ds`
            function ${context.name}(id: UserId): Result<User, Error> {
                const cached = cache.get(id);
                if (cached != undefined) {
                    return cached;
                }

                const user = ${state.innerName}(id)?;
                cache.set(id, user, ${state.capacity});
                return Result.ok(user);
            }
        `);

        context.replaceTarget(implementation);
    }
}
```

## Memory

TypeScript, like many managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed on some (local) heap, and all value types are copied by default.
That is convenient and often what we want, but sometimes we need to take direct control of memory, whether for better performance, or just to express certain invariants in the code.

Destack supports explicit, optional type modifiers for controlling memory _placement_ and _ownership_:
- **Placement** - where the value is located: ambient by default, explicitly `local` to one Worker, `shared` across Workers (or `static` for constant and `frame` for activation frames).
- **Ownership** - who "owns" the value: managed (`T`), owned (`^T`), borrowed (`&T`, `&readonly T`, `&exclusive T`), or raw (`*T`).

The two axes of ownership and placement compose and commute freely, e.g. `shared ^T` and `^shared T` both mean an owned handle to a value in shared space, and `shared &T` is a borrow of a shared value.
Once structural shapes and constraints have been [reified, instantiated, or erased](#representation), the plain old `T` still behaves as the type's default representation, exactly like we're used to from TypeScript: value types are values, object types are managed references, both can be aliased and mutated freely (locally).

Everything in this chapter follows from four rules:

1. Every value has exactly one owner - for managed values, the owner is the runtime.
2. A borrow must remain valid: a place cannot move or drop while an overlapping borrow is live, and a borrow cannot outlive the source it came from.
3. Exclusive borrows are truly exclusive: no overlapping borrow of the same place may be live.
4. Shared memory must never point into local memory.

### Ownership

Ownership determines who keeps a value alive, who is allowed to mutate it, and when and how it is eventually freed.
The notion of ownership has many names and forms, but today it is most commonly associated with Rust's explicit ownership system, and that is also the system most similar to Destack.
Really, "ownership" just means that each _value_ has an owner and certain rules apply to how we can pass and store values to certain places, depending on which level of mutability and ownership they need.

The usual explanation of "ownership" sounds more complex than it is, especially to developers used to "managed" languages, and _especially_ because Rust tradition (deliberately) unifies "liveness", "exclusivity" and "mutability" while only liveness is required for memory safety.
Unlike Rust, Destack supports _both_ multiple mutable borrows (`&T`) and exclusive mutable borrows (`&exclusive T`):

| Form | Meaning | Mutable? | Exclusive? |
|------|---------|----------|------------|
| `T` | normal managed/default value | yes | no |
| `^T` | owned value | yes | yes (single owner) |
| `&T` | borrowed access | yes | no |
| `&readonly T` | readonly borrowed access | no | no |
| `&exclusive T` | exclusive borrowed access | yes | yes |
| `*T` | raw pointer | yes (unchecked) | no (unchecked) |

The owner of a value is responsible for keeping it alive, and also for disposing of the owned value when the parent's own lifetime ends.
The parent (owner) could be in static storage for global constants, it could be another owned or even managed value, or it could be a call frame in a method (in which case we get a stack allocation).

```ds
let user: User = new User();   // managed: the runtime owns it
let owned: ^User = new User(); // owned: this binding owns it, dropped after last use

let borrow: &User = &user;                    // mutable borrow of the managed value
let view: &readonly User = &readonly user;    // OK: readonly may overlap the mutable borrow
let claim: &exclusive User = &exclusive user; // ERROR: `borrow` and `view` are still live
let pointer: *User = &user;                   // OK: raw pointers are inert and unchecked
```

Each ownership form also has a corresponding normalized representation in our little ["type algebra"](#algebra), which means we get to do regular TypeScript-style type space logic, conditionals and remapping (including for lifetimes!).

### Borrowing

To ensure memory safety even in unmanaged land, Destack follows the Rust idea of using ownership rules to ensure that _borrowing_ a reference `&T` remains valid - that is, `T` must remain alive (must not be deallocated) while _any_ `&T` is active.
Much more so than in Rust, Destack infers lifetimes for borrowed values even with complex control flow, so most of the time all lifetimes are inferred correctly and we don't need to think too much.

Unlike in Rust, in Destack, mutability is decoupled from borrowing: we can have multiple mutable borrows `&T` and readonly borrows `&readonly T` of the same `T` _at the same time_, as long as there is no concurrent `&exclusive T` borrow (which mirrors Rust's `&mut T`).
Importantly, this is still memory safe because all operations that may invalidate a borrow require `&exclusive T` access.

| Form | Access |
|------|--------|
| `&readonly T` | may overlap, cannot mutate through the borrow |
| `&T` | may overlap, can mutate through the borrow |
| `&exclusive T` | cannot overlap another borrow of the same place, can mutate through the borrow |

Borrow checking is just rule 2 applied per "access path", so disjoint fields can be borrowed independently when the compiler can prove they do not overlap.

```ds
struct Point {
    x: int32;
    y: int32;
}

class User {
    name: string = "";
}

let user: User = new User();

// `&user.name` borrows through the managed `User` handle
let name = &user.name;

// `name` is borrowed access into managed storage
name satisfies &string;

let point = ^Point { x: 1, y: 2 };

// `&readonly point.x` borrows from owned storage without moving `point`
let readX = &readonly point.x;

// `&point.x` *can* overlap with readonly borrowed access
let writeX = &point.x;
*writeX = 3;

// `&point.y` mutably borrows a disjoint field
let writeY = &point.y;
*writeY = 3;

// the readonly borrow is still valid here
readX satisfies &readonly int32;

// `&exclusive point.x` is allowed after the overlapping borrows are no longer live
let exclusiveX = &exclusive point.x;
*exclusiveX = 4;
```

#### Stability

Allowing multiple live mutable borrows are memory safe only because every operation that may _invalidate_ another live borrow requires exclusivity.
Writing through a non-exclusive borrow is therefore allowed only when the place is **overwrite-stable**: the old value needs no destruction (no drop glue), and the new bytes mean what the old bytes meant (one fixed layout, no variant tag) - which scalar and managed-reference fields trivially satisfy.
Everything else requires `&exclusive`: overwriting a variant reinterprets the payload under a live interior borrow, overwriting an owning value frees memory a borrow may still target, and so on.

```ds
struct Frame {
    pixels: ^Buffer; // owned interior storage
}

let frame: ^Frame = Frame { pixels: Buffer.open() };

let pixels = &frame.pixels; // interior borrow into the owned buffer
let alias = &frame;         // non-exclusive borrows may overlap

*alias = Frame { pixels: Buffer.open() }; // ERROR: overwrite drops the old buffer under `pixels`
```

It should be noted that as with the rest of borrowing and ownership, _regular_ managed land needs none of this because managed handles alias freely through the heap.
Writes through one managed handle can at most result in _stale_ reads through another (a "borrow" into an array taken before it grew still reads the old buffer), which is just ordinary TypeScript aliasing, and not really a memory safety problem in itself.

### Lifetimes

Lifetimes tie a borrow to its source, and in Destack they are just generics: `<comptime L: Lifetime>` parameters on `Borrowed<T, L>`, available to all the regular TypeScript-style type algebra and inference (including flow typing and narrowing).
Like `Access`, `Space`, and `Place`, `Lifetime` is a kind of static _value_ rather than a type, which is why parameters over these forms always carry the `comptime` modifier.
In practice they are spelled out in exactly one place - `declare` signatures - and inferred everywhere else, including from function bodies.

```ds
function read(user: &User): &string {
    return &user.name;
}

function first<T>(items: &[T]): &T {
    return &items[0];
}
```

Elided borrowed forms get hidden generic lifetime parameters in _all_ declarations, not just in function signatures.

```ds
// elided form
struct WorldView {
    engine: &Engine;
    assets: &AssetStore;
}

// explicit form
struct WorldView<comptime L1: Lifetime, comptime L2: Lifetime> {
    engine: Borrowed<Engine, L1>;
    assets: Borrowed<AssetStore, L2>;
}
```

Because lifetimes are part of type inference, and type inference also analyzes method bodies, lifetime inference also derives from function bodies.
Each elided borrow in the signature induces its own hidden lifetime parameter first, and the body then solves the return lifetime:

```ds
// elided form
function first(a: &Node, b: &Node): &Node {
    return a;
}

// explicit form
function first<comptime L1: Lifetime, comptime L2: Lifetime>(
    a: Borrowed<Node, L1>,
    b: Borrowed<Node, L2>,
): Borrowed<Node, L1> {
    return a;
}

// elided form
function choose(a: &Node, b: &Node, flag: boolean): &Node {
    return flag ? a : b;
}

// explicit form
function choose<comptime L1: Lifetime, comptime L2: Lifetime>(
    a: Borrowed<Node, L1>,
    b: Borrowed<Node, L2>,
    flag: boolean,
): Borrowed<Node, L1 | L2> {
    return flag ? a : b;
}
```

Because `declare` functions do not have bodies, it follows that declaration-only APIs must spell out lifetime relationships explicitly.

```ds
// rejected
declare function only(value: &Node): &Node;

// accepted
declare function only<comptime L: Lifetime>(value: Borrowed<Node, L>): Borrowed<Node, L>;

// rejected
declare function choose(
    a: &Node,
    b: &Node,
): &Node;

// accepted
declare function choose<comptime L1: Lifetime, comptime L2: Lifetime>(
    a: Borrowed<Node, L1>,
    b: Borrowed<Node, L2>,
): Borrowed<Node, L1 | L2>;
```

Taken together, the mechanisms of elision and inference for lifetimes let us implement the vast majority of low level ownership patterns _without_ having to specify lifetimes to the compiler explicitly (yay).
It should also be noted that one consequence of this body-derived inference is that a body change can change an inferred public signature, but that is just the tradeoff here.

### Suspension

As discussed in [Continuations](#continuations), values and objects in "managed land" work exactly like in regular TypeScript and can be referenced and mutated freely wherever.
When _borrowing_ across suspension points (like `yield` or `await`), the compiler must still guarantee that the core safety rules are upheld.
And because multiple continuations may be active concurrently (even if not in parallel), borrows that must be owned by the current frame to cross suspension points - in other words, borrows coming from managed values must _not_ cross suspension points.

It follows that the same body is fine or rejected purely based on where the borrowed value lives:

```ds
// owned by the frame: the borrow may cross suspension
async function readOwned(user: ^User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}

// borrowed parameter: also fine, the caller proves the source is owned or static
async function readBorrowed(user: &User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}

// managed: rejected, another continuation could touch `user` while we are parked
async function readManaged(user: User): Promise<string> {
    const name = &readonly user.name;
    await tick(); // ERROR: `name` borrows managed storage across suspension
    return name.clone();
}
```

One load-bearing guarantee underneath all of this: suspension points are always lexically explicit (`await` and `yield`).
There is no hidden suspension - no implicit awaits, no preemption points, no suspending allocators - so "does this borrow cross suspension" can always be answered by looking at the code (and declaration-only APIs need no extra annotations, since asyncness is visible in signatures and `declare` signatures already spell out lifetimes).

### Drop

Whenever the lifetime of a value ends and it is deallocated, Destack supports running a `Drop` finalizer, similar to Rust's `Drop`.
This happens when the compiler inserts a drop for an owned local after its last use, when an owned field is being destroyed, and when the runtime reclaims an unreachable managed allocation.
Drop sites are statically known: a place conditionally moved on one branch is an error at the join, so there are no runtime drop flags, and drops lower to plain calls.

```ds
function run(): void {
    let buffer: ^Buffer = Buffer.open();
    process(&buffer);
    // `buffer` is dropped immediately after its last use
}
```

The same `Drop` also works for managed references, where it is run before they are freed:

```ds
class Image {
    pixels: Unique<[uint8]>;
}

let image: Image = new Image();
// when `image` is collected, the GC runs drop glue for `pixels`
```

Unlike Rust and C++ RAII, Destack models `Drop` and `Dispose` / `AsyncDispose` separately: `Drop` follows lifetime, while [`using`](#using) is lexical.
Because of this split, `Drop` _can_ run eagerly after last use, which is great for memory pressure, but also means Destack's `Drop` is not the right fit for lexical RAII-style cleanup.
In general, `Drop` should manage memory and memory-shaped cleanup, while `using` should manage richer resource finalization:

| Protocol | Purpose | Timing |
|----------|---------|--------|
| `Drop` | ownership finalization for memory and owned fields | deterministic for owned values, GC-timed for managed values |
| `Dispose` | explicit synchronous resource cleanup | lexical `using` scope exit |
| `AsyncDispose` | explicit asynchronous resource cleanup | lexical `await using` scope exit |

External resources such as files, locks, sockets, transactions, and temporary runtime registrations should use `Dispose` or `AsyncDispose`, not `Drop`.
This keeps RAII-style cleanup explicit and predictable even though ordinary TS++ values are "managed" by default:

```ds
using file = File.open(path)?;
await using connection = await pool.connect();
```

Low-level code can of course still control finalization explicitly:
`drop(value)` ends ownership immediately, `forget(value)` intentionally suppresses automatic drop, `ManuallyDrop<T>` stores a value outside automatic drop handling, and `Box<T>.leak()` turns one owned allocation into a static borrow.

### Allocation

The primary way to construct new values is `new`, which initializes a `T` and produces the ownership form required by the _destination_ type - `new` is really just an initializer that calls the type's constructor.
The required destination type decides whether that is managed storage, owned storage, inline frame storage, shared storage, or some lower-level allocation form.

```ds
let a: User = new User();   // managed
let b: ^User = new User();  // owned
```

In some situations it is useful to handle allocation errors directly, and for that Destack supports more direct fallible allocation accessors both via higher level `try*` methods in standard library types (like `Array.tryReserve`) and the low level intrinsics (`MaybeUninit<T>`, `Unique<T>`, etc.) that they are built on.
Regular managed object allocation (`new T()`) can also be made fallible via the `new?` operator, which behaves similar in spirit to the regular [`?`](#maybe-must-and-coalesce) and `await?` operators, propagating an `AllocationError` to the containing context:

```ds
try {
    let buffer: Block[] = Array.tryWithCapacity(count)?; // fallible library allocation
    let page = new? Page(buffer);                        // fallible managed allocation
    page.fill()?;

    return page;
} catch (e: AllocationError) {
    return null;
}
```

Code can also opt out of managed allocation locally via [restrictions](#restrictions):

```ds
@noManaged
function processFrame(input: &[Sample]): ^Frame {
    return buildFrame(input);
}

@noHeap
function interruptHandler(input: &[Sample]): Frame {
    return buildFrameOnStack(input);
}
```

### Space

The space of a value determines where it is actually located in memory, and since Destack follows web and TypeScript conventions, the `Worker`-_local_ heap is the default main memory space.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space, and user and library code can almost always just pretend spaces don't even exist.

```ds
struct Request<T> {
    header: Header;
    body: T;
}

let localRequest: Request<Body>;          // ambient, default -> Request is worker-local heap
let sharedRequest: shared Request<Body>;  // explicit, shared -> Request is shared heap
```

Memory placement is contextual: all types are "ambient" by default, i.e., they come with no inherent placement, and types are only placed wherever their parent is placed until someone either specifies placement explicitly (e.g., `local T`, `shared T`) or we reach the top, which - as established - is `local` to the Worker's own local heap by default.
This "ambient placement" rule is also why we distinguish `Place` from `Space`: `Space` is concrete, while `Place` may also be `"ambient"`.

#### Shared Space

In TypeScript tradition, each "Worker" (in the JS spec generally referred to as "Agent") has its own isolated local heap that cannot be touched by other workers.
For sharing memory across workers, TypeScript has the `SharedArrayBuffer` concept and plain old message passing with `postMessage`.
That works, but is architecturally limited and makes it complex to implement more sophisticated parallelism patterns.

Destack supports an explicit, separate, fully featured _shared_ memory space that is visible to all `Worker`s in the same `Runtime` via regular references and objects.
Basically, `shared T` is the typed, generalized version of the `SharedArrayBuffer` idea with the full type system and object graphs at our disposal with the simple rule that local may point into shared, but shared must not point into local memory.

It's important to note that _by itself_ shared placement, like any space placement, is **not** a synchronization primitive and does **not** imply atomic access, locking, `Sync`, or anything like that.
That is by design; it's up to userland libraries to require [capabilities](#capabilities) like `Send` and `Sync` for APIs that transfer or publish values for correctness, but `shared` itself is really only about placement.

#### Static Space

Because Destack inherits the JS/TS Worker model for isolation, module-scoped constants are owned by each _Worker_ and are not actually process-global as they would be in most other languages.
For genuinely _shared_ process-global state, the binding _itself_ can be declared as `shared`.

| Form | Binding place | Value place | Meaning |
|------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

Note that marking the binding itself as `shared` also types the value as `shared` (as it is illegal to point from shared storage into local storage anyway, this is convenient).
Shared bindings are always `const`, and `const` means exactly what it means in TypeScript: the binding is not reassignable, while the value behind it mutates freely through its own API.

One constraint follows directly from the Worker model: a `shared` binding is reachable from _every_ Worker by construction, so it must meet the same bar as any value crossing Workers - the value type must be `Sync` (see [Capabilities](#capabilities)).
In particular, a bare owned global like `shared const world: ^World` is rejected: every Worker can reach the binding, so its exclusivity cannot be checked.
Global mutable state instead puts a lock in the static and lets the guard be the runtime exclusivity oracle:

```ds
shared const world: Mutex<World> = new Mutex(new World());

const view = world.lock(); // guard dereferences to &exclusive World
```

Because module code runs on every Worker, shared bindings are initialized exactly once by the runtime, before any other Worker can observe them.

### Conversions

Reference conversions follow directly from [the four memory rules](#memory), and borrowing from a live place works whenever the requested loan rules hold:

| Source | Borrow | Notes |
| --- | --- | --- |
| local managed `T` | `&T` / `&readonly T` / `&exclusive T` | exclusive needs no overlapping loan; none may cross [suspension](#suspension) |
| owned `^T` | `&T` / `&readonly T` / `&exclusive T` | owned sources may also cross suspension |
| shared owned `shared ^T` | `shared &T` / `shared &readonly T` / `shared &exclusive T` | owned storage stays unique even in shared space, since the handle itself still has one holder |
| shared managed `T` | `&T` / `&readonly T` | mutation routes through `Sync` APIs and atomic field access (see [Synchronization](#synchronization)) |
| shared managed `T` | `&exclusive T` | never: a shared managed handle cannot prove uniqueness |

Borrows can weaken freely, but cannot be upgraded (obviously):

| From | To | Notes |
| --- | --- | --- |
| `&exclusive T` | `&T` / `&readonly T` | temporary reborrow that suspends the exclusive loan |
| `&T` | `&readonly T` | readonly reborrow |
| `T` | `^T` | never: managed ownership does not become unique ownership |
| `&T` | `T` / `^T` | never: borrowed access does not own the value |

The missing `T → ^T` rung is deliberate: ownership is provenance, not a view, and a traced GC cannot even count references to check uniqueness at runtime.
The bridges are explicit instead: construct owned from the start (`new` already produces the destination form), take `MaybeOwned<T>` when an API wants ownership but tolerates managed callers (branch once, use or clone), and `.clone()` when what you actually need is a copy.

Raw pointers convert freely in, and only unsafely out:

```ds
let user = new User();

let borrow: &User = &user;   // default: a checked borrow
let pointer: *User = &user;  // typed as raw: an inert, unchecked pointer value
let again: &User = pointer;  // ERROR: pointers only reborrow inside @unsafe
```

### Unsafe

Safe Destack code can create and carry raw pointers, because there is nothing directly unsafe about just looking at pointers.
Raw pointers are inert: they do not keep storage alive, do not participate in borrow checking, and do not prove exclusivity.

Converting a borrow to a raw pointer is still safe because it does not touch the pointed-to memory:

```ds
let user = new User();

let borrow: &User = &user;
let pointer: *User = borrow; // OK: this only creates a raw pointer value
```

Unsafe begins when code relies on a memory invariant the compiler cannot prove:

| Operation | Example | Safe? | Why |
|-----------|---------|-------|-----|
| create or carry raw pointer values | `let pointer: *User = &user`, `pointer == other` | yes | does not touch memory |
| reinterpret raw pointer values | `pointer as *uint8`, `0x1000 as *uint8` | yes | makes no validity claim |
| wrapping address arithmetic | `wrappingOffset(pointer, 4)` | yes | makes no allocation claim |
| allocation-relative pointer math | `offset(pointer, 4)`, `offsetFrom(pointer, origin)` | no | claims same live allocation |
| access memory through a pointer | `asReference(pointer)`, `read(pointer)`, `write(pointer, value)` | no | bypasses borrow checking |
| build typed views from raw storage | `Slice.fromRaw(pointer, length)` | no | claims a valid region of `T` |
| raw bytes and layout tricks | `copyBytes(dst, src, n)`, `readVolatile(pointer)`, `transmute<T, U>(value)` | no | touches or reinterprets unchecked memory |

The compiler rejects unsafe operations, like raw pointer dereferencing, outside explicit [`@unsafe` / `@safe`](#taint) contexts.

### Algebra

Destack's "memory algebra" is a fancy way of saying that the axes of ownership, access, lifetime, and placement are just types that we can do TypeScript-style algebra and inference with.
Code can inspect a type's memory form and build a derived form because all the qualified surface forms like `readonly T`, `^T`, `&T`, `*T`, `local T`, and `shared T` correspond to builtin intrinsic types:

```ds
/// Automatically managed T, owned by the runtime.
newtype Managed<T> = intrinsic;
/// Owned T (`^T`).
newtype Owned<T> = intrinsic;
/// Borrowed T (`&T`).
newtype Borrowed<T, comptime L: Lifetime, comptime A: Access = "mutable"> = intrinsic;
/// Raw T (`*T`).
newtype Raw<T> = intrinsic;
/// Placed T (`local T` or `shared T`).
newtype Placed<T, P: Place> = intrinsic;
/// Readonly T (`readonly T`).
newtype Readonly<T> = intrinsic;
```

Specifically, all surface sigils and keywords are just compact syntax for those intrinsic forms that commute the way they read:

```ds
User            // unqualified, normal default representation
readonly User   // Readonly<User>
^User           // Owned<User>
&readonly User  // Borrowed<User, L, "readonly">
&User           // Borrowed<User, L, "mutable">
&exclusive User // Borrowed<User, L, "exclusive">
*User           // Raw<User>
local User      // Placed<User, "local">
shared User     // Placed<User, "shared">
local ^User     // Placed<Owned<User>, "local">
shared ^User    // Placed<Owned<User>, "shared">
^shared User    // Owned<Placed<User, "shared">>
```

It follows that because forms compose, owning a borrow is different from borrowing an owner:

```ds
Owned<Borrowed<User, L>> // owns a borrow value
Borrowed<Owned<User>, L> // borrows an owned value
```

Destack also provides builtin accessors to inspect composed forms, e.g., `PayloadOf<T>` removes one outer form, while `BaseOf<T>` removes all transparent memory forms:

```ds
BaseOf<shared ^User> satisfies User;
PayloadOf<Owned<Borrowed<User, L>>> satisfies Borrowed<User, L>;
OwnershipOf<^User> satisfies "owned";
OwnershipOf<Owned<Borrowed<User, L>>> satisfies "owned";
OwnershipOf<Borrowed<Owned<User>, L>> satisfies "borrowed";
OwnershipOr<User, "managed"> satisfies "managed";
AccessOf<User> satisfies "mutable";
AccessOf<readonly User> satisfies "readonly";
AccessOf<^readonly User> satisfies "readonly";
AccessOf<&exclusive User> satisfies "exclusive";
```

`Space` represents one builtin concrete space while `Place` means either a concrete `Space` or `"ambient"`, and ambient placement follows the containing context until a final layout is required:
```ds
PlaceOf<User> satisfies "ambient";
SpaceOf<^User> satisfies never;
PlaceIn<^User, "shared"> satisfies "shared";

PlaceOf<local User> satisfies "local";
SpaceOf<local User> satisfies "local";
PlaceIn<local User, "shared"> satisfies "local";

PlaceOf<shared User> satisfies "shared";
SpaceOf<shared User> satisfies "shared";
PlaceIn<shared User, "local"> satisfies "shared";

PlaceOf<User | local User | shared User> satisfies "ambient" | "local" | "shared";
SpaceOf<User | local User | shared User> satisfies "local" | "shared";
PlaceIn<User | local User | shared User, "local"> satisfies "local" | "shared";
PlaceIn<User | local User | shared User, "shared"> satisfies "local" | "shared";
```

Predicates with `Is*` are convenience wrappers around those same accessors:

```ds
IsOwned<^User> satisfies true;
IsBorrowed<&User> satisfies true;
IsShared<shared User> satisfies true;
IsShared<^User> satisfies false;
IsSharedIn<^User, "shared"> satisfies true;
```

Rewriting helpers preserve the rest of the type instead of rebuilding from a stripped base type:

```ds
WithSpace<^User, "shared"> satisfies Placed<^User, "shared">;
WithOwnership<shared User, "owned"> satisfies Owned<shared User>;
WithPlace<shared User, "ambient"> satisfies Placed<User, "ambient">;
WithAccess<User, "readonly"> satisfies readonly User;
WithAccess<^User, "readonly"> satisfies ^readonly User;
WithAccess<&User, "exclusive"> satisfies &exclusive User;
```

Because memory qualification is just ordinary type-level computation, userland code can introspect and rewrite ownership and placement using the same type system we already use for all other types.
Inside a type declaration, `this` in type or static position also carries the current instantiated form of that type to query against with the `*Of` and `Is*` family.

```ds
struct Buffer<T> {
    comptime const IsShared = PlaceOf<this> == "shared";

    @if(this.IsShared)
    lock: Mutex;

    value: T;
}

declare const localBuffer: Buffer<string>;
declare const sharedBuffer: shared Buffer<string>;

sharedBuffer.lock satisfies Mutex;
PlaceOf<typeof localBuffer> satisfies "ambient";
PlaceOf<typeof sharedBuffer> satisfies "shared";
```

### Polymorphism

Since ownership, access, lifetime, and placement are all reified as memory form types, contracts and implementors get to be polymorphic and (somewhat) conditional over their ownership, space, and access, even on the receiver type.
That lets types expose one natural operation when only the projected form changes, and separate operations when the semantics actually differ.
The caller chooses the level of control by writing the expression / providing the type they mean:

```ds
process(user);            // managed/default value
process(&readonly user);  // readonly borrowed access
process(&user);           // mutable borrowed access
process(&exclusive user); // exclusive borrowed access
process(^user);           // owned value
```

Dispatch resolution uses the actual form during overload resolution, so container interfaces like `Iterable<T>` can support ordinary TypeScript iteration and borrowed iteration without adding Rust-style method family explosion:

```ds
declare type Point = { x: number; y: number };
declare const points: Array<Point>;

// ordinary "managed" iteration
for (const point of points) {
    point satisfies Point;
}

// readonly borrowed iteration
for (const point of &readonly points) {
    point satisfies &readonly Point;
}

// mutable borrowed iteration
for (const point of &points) {
    point satisfies &Point;
}

// exclusive borrowed iteration
for (const point of &exclusive points) {
    point satisfies &exclusive Point;
}

// moved iteration
for (const point of ^points) {
    point satisfies Point;
}
```

### Capabilities

Inspired by many other systems-y languages, Destack encodes synchronization and memory primitives as trait-like interfaces like `Copy`, `Clone`, `Send`, and `Sync`.

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |
| `Send` | Value can cross a Worker boundary. |
| `Sync` | References to shared values can be used concurrently through the type's own API. |

Like Rust's auto traits, `Send` and `Sync` are derived structurally by default (a type is `Send` when all of its fields are) and can be opted out of explicitly.
Implementing `Send` or `Sync` _manually_ - against the structural evidence, as interior-mutability primitives must - is an `@unsafe` claim like any other unchecked memory assertion.
For borrows and handles, the rules follow per memory form:

| Form | Crosses Workers when |
| --- | --- |
| `T` (local managed) | never, local handles stay on their Worker |
| `shared T` | `T: Sync` for aliased access |
| `^T` | `T: Send` |
| `&readonly T` | `T: Sync`, and the source is owned or static |
| `&exclusive T` | `T: Send`, and the source is owned or static |
| `&T` | never, aliased mutability |
| `shared` static binding | requires `T: Sync` outright, the binding reaches every Worker by construction |

The `&T` row is the important one: the aliased-mutable middle of the borrow ladder is only safe because a Worker is a single-threaded execution domain, and so it must never cross a Worker boundary.
Also, note again `shared T` means `T` lives in [shared space](#shared-space); it does _not_ make `T` automatically `Sync` by itself.
Userland APIs such as channels, Worker pools, atomics, locks, and actors can require `Send` or `Sync` when they need those stronger guarantees, very similar to Rust or even Swift.

### Synchronization

The standard library provides the usual memory and synchronization primitives on top of this unified memory system.
The full details are documented in the library, but the basics should be familiar to anyone with a systems-level background.
This is also where the `@unsafe` actually lives: a `Mutex<T>` mutates through a non-exclusive receiver via `UnsafeCell` and atomics, and its guard manufactures the `&exclusive T` that the runtime lock (not the borrow checker) guarantees - userland just composes the safe surface.

| Primitive | Contract |
|-----------|----------|
| `Box<T>` | unique heap ownership for `Owned<T>`, with deterministic drop when `T: Drop` |
| `Rc<T>` | local shared ownership of `Owned<T>`, non-atomic refcount, not transferable across Workers |
| `Arc<T>` | shared ownership of `Owned<T>`, atomic refcount, transferable when `T` satisfies the required `Send` / `Sync` bounds |
| `Cell<T>` | local interior mutation by value, for small `Copy`-like state |
| `RefCell<T>` | local runtime borrow checking for cases static borrowing cannot express cleanly |
| `Atomic<T>` | lock-free scalar storage with explicit ordering and scope |
| `AsyncMutex<T>` | local mutual exclusion that suspends the current async task, not the Worker |
| `Mutex<T>` | shared mutual exclusion backed by atomics and runtime wait/wake support |

Because ownership and placement are part of our type system, and we can query and gate based on contextual type information using regular TypeScript algebra, we have a lot of flexibility and gain some nice ergonomics.
For example, we can provide ergonomic context-aware aliases that conform to the way they are used:

```ds
type Ref<T> =
    PlaceIn<T, "local"> extends "shared" ? Arc<T> : Rc<T>;

type Lock<T> =
    PlaceIn<T, "local"> extends "shared" ? Mutex<T> : AsyncMutex<T>;
```

# Runtime

The runtime is where code actually _runs_, and it's where pure computation touches the real world via our well defined `host` bindings.
This is nice because it means we get to capture and analyze all effects through a relatively thin well known boundary, which enables great observability and debugging.
And because Destack is a fully integrated stack, the runtime has been co-designed as part of the entire language toolchain, and the standard library takes advantage of this.

## Modules

Destack's module system goes a little further than plain code imports: sources can be included conditionally, additional file types import as typed modules, and modules carry metadata.

### Conditions

Conditions generalize the idea behind `module.tests.ds` and `[cfg(attr)]`-style feature gating into a flexible "condition system" for, well, conditionally including sources and parts of sources into some builds (but not others).
We recognize `conditions` of `mode`, `feature`, `role`, `stage`, and a general `tag`, in addition to all the usual target gates (e.g. `host`, `runtime`, `target`, `platform`):

```json:destack.json
{
    "stage": "alpha",
    "conditions": {
        "modes": {
            "test": {},
            "dev": {},
            "prod": {},
            "preview": { "extends": "dev" }
        },
        "features": {
            "rendererv2": {}
        },
        "roles": {
            "server": {},
            "client": {}
        },
        "aliases": {
            "alpha": { "stage": "alpha" },
            "browser": { "host": "browser" }
        }
    },
    "compiler": {
        "modes": ["preview"],
        "features": ["rendererv2"]
    }
}
```

One stage is active at a time - and profiles, products, and targets may override it for the resolved profile.
The active conditions are available within code via [`import.meta.<condition>`](#import-meta) (like `import.meta.roles`) as usual for in-code dynamic gating:

```ds
function getRenderer(): Renderer {
    if (import.meta.features.includes("rendererv2")) {
        return new RendererV2();
    } else {
        return new RendererV1();
    }
}
```

On the import side, Destack also generalizes `<module>.<alias>.ds` to support conditional inclusion of files (appended to the `module.ds` file itself) based on the active conditions via `aliases`.
Every named condition is automatically available as an `alias`, and additional `aliases` can be declared explicitly in `"conditions"` based on target-shaped gates.
For example, when importing `./user` with `test` mode active, both `user.ds` and `user.test.ds` are included (as if):

```ds
// user.ds
export function loadUser(id: UserId): Result<User, UserError> {
    return database.load(id);
}

// user.test.ds
test("loadUser", () => {
    loadUser(UserId(1)) satisfies Result<User, UserError>;
});
```

Chained names like `user.test.browser.ds` behave as "and" gates on all conditions, that is, `user.test.browser.ds` is included only when both `test` mode and the `browser` alias match.
Package dependencies can also be gated by the same condition system.
Top-level dependencies are always part of the source graph, while `conditionalDependencies` are included when their `when` predicate matches:

```json:destack.json
{
    "dependencies": {
        "@destack/http": { "source": "registry", "version": "^1.0.0" }
    },
    "conditionalDependencies": [
        {
            "when": "test",
            "dependencies": {
                "@destack/test": { "source": "registry", "version": "^1.0.0" }
            }
        },
        {
            "when": { "feature": "sqlite", "host": "native" },
            "dependencies": {
                "@destack/sqlite": { "source": "registry", "version": "^1.0.0" }
            }
        }
    ]
}
```

### Import Meta

`import.meta` exposes profile metadata and current module metadata during static and comptime evaluation.

| Field | Description | Type | Examples |
|-------|-------------|------|----------|
| `import.meta.url` | current module URL | `string` | `"file:///app/src/main.ds"`, `"https://example.com/mod.ds"` |
| `import.meta.path` | current local file path, when available | `string | undefined` | `"/app/src/main.ds"`, `undefined` |
| `import.meta.dir` | current local directory, when available | `string | undefined` | `"/app/src"`, `undefined` |
| `import.meta.output` | output artifact format | `Output` | `"js"`, `"wasm"`, `"native"` |
| `import.meta.platform` | target operating system | `Platform` | `"linux"`, `"windows"`, `"none"` |
| `import.meta.host` | target host environment | `Host` | `"browser"`, `"native"`, `"wasi"` |
| `import.meta.target` | target family and ABI | `Target` | `{ family: "unix", arch: "x64", abi: "gnu" }` |
| `import.meta.targetName` | active build target name | `string | undefined` | `"web"`, `"native"` |
| `import.meta.product` | active deliverable product name | `Product | undefined` | `"app"`, `"server"` |
| `import.meta.version` | active package version | `string | undefined` | `"2026.5.27-alpha.1"` |
| `import.meta.stage` | active package release stage | `Stage | undefined` | `"alpha"`, `"stable"` |
| `import.meta.runtime` | semantic runtime | `Runtime` | `"destack"`, `"js"` |
| `import.meta.modes` | active source graph modes | `readonly Mode[]` | `["test"]`, `["dev", "lint"]` |
| `import.meta.roles` | active source graph roles | `readonly Role[]` | `["server"]`, `["client"]` |
| `import.meta.features` | active source graph features | `readonly Feature[]` | `["checkout"]`, `["renderer"]` |
| `import.meta.tags` | active source graph tags | `readonly Tag[]` | `["preview"]`, `["internal"]` |
| `import.meta.<mode>` | mode shorthands for `debug`, `dev`, `prod`, `test`, `bench`, `lint` | `boolean` | `import.meta.test`, `import.meta.prod` |
| `import.meta.env` | configured build environment | `{ readonly [key: string]: string | boolean | number }` | `{ NODE_ENV: "production", FEATURE_X: true }` |
| `import.meta.tree` | current module tree tag builder | `TreeTagBuilder | undefined` | `HtmlTree` |
| `import.meta.derive` | current module auto derives | `readonly Derive[]` | `["Clone", "Debug"]` |
| `import.meta.labels` | current module labels | `{ readonly [key: string]: unknown }` | `{ feature: ["checkout"] }` |

### Data Modules

Data files are parsed at compile time and typed as exact readonly literals by default:

```json:config.json
{
    "server": {
        "host": "127.0.0.1",
        "port": 8080
    },
    "debug": false
}
```

```ds:main.ds
import config from "./config.json";

config.server.host satisfies "127.0.0.1";
config.server.port satisfies 8080;
config.debug satisfies false;
```

Types are inferred from the data:
- `null` → `null`
- `true`/`false` → `true` / `false`
- Numbers → numeric literal types
- Strings → string literal types
- Arrays → readonly tuples
- Objects → readonly object literals

Consumers can widen explicitly with an annotation or conversion:

```ds
const general: Config = config;
```

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

## Policy

Destack supports configuring the policy that controls which host actions some piece of code - like a module or some dependency - may perform.
Packages declare the actions and resources they require, and the app or workspace decides which requirements are allowed.

```json:destack.json
{
    "policy": {
        "requires": [
            { "action": "fs.read", "resource": "app://config/**" },
            { "action": "net.connect", "resource": "tcp://database.internal:5432" }
        ],
        "rules": [
            {
                "subject": { "package": "app" },
                "action": "fs.read",
                "resource": "app://config/**",
                "access": "allow"
            },
            {
                "subject": { "package": "@vendor/parser" },
                "action": "net.connect",
                "resource": "*",
                "access": "deny"
            }
        ]
    }
}
```
