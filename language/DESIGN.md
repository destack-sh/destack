# "Language"

The Destack language (`.ds`) and toolchain, colloquially "TypeScript++", are a superset of "strict modern" TypeScript with support for `.ts` and `.tsx` files, native AOT compilation and a fully integrated language toolchain, _and_ it can also compile nicely to standard JS/TS targets.
We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated stack (Destack), and thus by "language" ("TypeScript++") we mean much more than "just" a syntax form: a language, a runtime, a toolchain, plugins, and ultimately, a way of programming.

## Universality

We're very early in software, and we're still figuring out how to build optimal, correct, and integrated software systems.
Over 50 years, we have grown more and more layers of software sediment and need ever _more_ tools to get any code out the door, and yet confidence and performance have plummeted.
We can do better, but not by adding _more_ and more inscrutable pieces.

The best possible stack must be fully integrated across the language itself, the toolchain with linters and formatters, a VM, compiler, runtime, and basically anything that touches the code.
Only TypeScript is close to being a universal software foundation, because it runs directly on the web, and the web is the most ubiquitous application platform.
The TypeScript ecosystem has good - if not perfect - answers to all modern software needs, from great developer tools to rich interactive frontends to quite _decent_ and performant backends.

Excluding legacy JavaScript baggage and dynamic prototype mess, modern TypeScript is surprisingly close to a fully AOT-compilable language (and most browsers retrofit compilation internally already based on these assumptions).
Embracing TypeScript and "the web ecosystem" lets us build a new toolchain that truly covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be completely free of JS overhead and (some) historic baggage.

## Compatibility

**Destack is a superset of the "modern strict" subset of TypeScript**.
The intended use case for Destack is making TS-shaped code optimal and correct, which requires cutting all accumulated dynamic magic that might smuggle in ambiguity or unsoundness at runtime.
Accordingly, Destack excludes legacy syntax and all sorts of dynamic shapes and protocols that are not statically sound, and also removes a few rarely used footguns.

### Modules

Destack supports only ESM syntax without namespaces, stringy-modules, and also does not distinguish "type" from "value" imports/exports.

| Feature | Example | Compatibility | Reason |
| --- | --- | --- | --- |
| **Type-only imports / exports** | `import type { User } from "./user"` | supported as aliases for `import` and `export` | Destack has one static module graph, and the type-less form is preferred |
| **CommonJS** | `require("x")`, `module.exports`, mutable `exports`, require-cache monkeypatching, `export = value`, `import x = require("x")` | not supported | CommonJS is a legacy mutable runtime module system |
| **String module declarations** | `declare module "pkg" { ... }` | not supported | source should declare real modules instead |
| **Namespace declarations** | `namespace Name { ... }` | not supported | use real modules with namespace imports or re-exports |
| **Dynamic module loading** | `import(expr)` | not source-level module loading | the source graph is statically known, though JS output may still use dynamic imports for chunk loading |
| **Import defer** | `import defer * as ns from "pkg"`, `import source x from "pkg"` | not supported | not a real ESM imports (also unclear semantics in AOT) |
| **Import assertions** | `import data from "./data.json" assert { type: "json" }` | not supported | use standardized import attributes with `with { ... }` |
| **String export names** | `export { value as "name" }` | not supported | export names should be identifiers in source modules |

### Syntax

Destack does not support JS/TS syntax that conflicts with either Destack-specific features (like `(A, B)` tuples over sequence expressions) or are just plain legacy like `<T>expr` type assertions.

| Feature | Example | Compatibility | Reason |
| --- | --- | --- | --- |
| **Var declarations** | `var x` | not supported | legacy `var` scoping is unnecessary with `const` and `let` |
| **Ambiguous generic arrow** | `<T>() => value` | not supported | ambiguous with TSX tree syntax, use `<T,>() => value` |
| **Sequence expressions** | `(a, b, c)` | not supported | `.ds` claims parenthesized comma lists for explicit tuples |
| **Single-quoted literals** | `'A'` | `char` in `.ds`, string in `.ts` / `.tsx` | `.ds` uses double-quoted strings and single-quoted scalar characters |
| **Type angle assertions** | `<T>value`, `<const>value` | legacy angle-bracket assertions are not supported | use `value as T`, `value satisfies T`, or `value as const` |
| **Non-null assertions** | `value!` | supported as Try / must unwrapping, not as erased TypeScript non-null assertion | non-null opening is an explicit runtime operation |
| **XML namespace resolution** | `<svg:path />` | no `xmlns` binding semantics | namespaced tree tags are intrinsic string tag names like `"svg:path"` |

### Types

Destack requires sound and predictable types and understands only TypeScript-shaped type syntax.

| Feature | Example | Compatibility | Reason |
| --- | --- | --- | --- |
| **Flow and JSDoc typing** | `/** @type {Foo} */` | ignored or rejected when not valid TS/TS++ | TypeScript type syntax only |
| **Import type queries** | `import("pkg").User` | not supported | use ordinary static imports instead |
| **Thenables** | `await customThenable` | not supported | `await` works on the well known `Promise<T>` only |
| **`any`** | `let x: any` | rejected in `.ds` | use `unknown`, which must be explicitly cast before use |
| **Definite assignment assertions** | `let x!: T`, `field!: T` | rejected in `.ds` | locals and fields must be initialized before use |
| **Generic argument ambiguity** | `Foo<{ value: string }>` | object-shaped type arguments need `type` | static value and type arguments share generic forms |
| **Circular inference** | mutually inferred module exports | not supported across modules | downstream uses do not refine upstream declarations |
| **Enum coercion** | `Level.A` as `number` | no implicit coercion | enum fields are nominal constants and need explicit conversion |

### Shapes

Destack requires sound static shapes for all object types and thus does not support prototype chains, dynamic declarations, or runtime mutation.

| Feature | Example | Compatibility | Reason |
| --- | --- | --- | --- |
| **Declaration expressions** | `const C = class {}` | not supported | runtime type generation is not statically knowable |
| **Prototype objects** | `.prototype`, `.__proto__`, `.constructor`, `Object.getPrototypeOf`, `Object.setPrototypeOf`, `Object.create(proto)` | not supported | prototypes rely on the dynamic JavaScript object model |
| **Shape mutation** | `delete obj.x`, `Object.defineProperty`, `Object.defineProperties`, `Reflect.defineProperty`, `Reflect.deleteProperty`, shape-changing `Object.assign` | forbidden | object shapes must stay statically known |
| **Metaobject dispatch** | `Proxy`, most `Reflect.*` APIs | not supported | dynamic interception and emulation hide object behavior from the static model |
| **Class index signatures** | `class C { [key: string]: T }` | not supported | classes have fixed declared members, use structural object types or interfaces instead |
| **Array holes** | `[1,,3]` | not supported | dense sequences make indexing and layout predictable |

### Runtime

Destack does not support any unsound, imprecise or dynamic legacy hooks into runtime behavior, and that also means exceptions are officially banned (try-catch-finally works with `Result` types though).

| Feature | Example | Compatibility | Reason |
| --- | --- | --- | --- |
| **Sloppy mode** | duplicate function declarations, `arguments` magic, `caller`, `callee`, `yield` identifiers, `with` | not supported | Destack targets modern strict-mode TypeScript |
| **Loose equality coercion** | `a == b`, `a != b` | object coercion is not allowed | implicit object conversion hides behavior |
| **Truthiness** | `if (value)` | only for boolean values | control flow must use explicit boolean tests |
| **Dynamic code generation** | runtime `eval`, `new Function`, dynamic class generation | unsupported, except explicit `comptime eval` | runtime code generation conflicts with AOT compilation |
| **Exceptions** | executing `throw` / `catch` effects | `throw`, `try`, `catch`, `finally`, and Try / Result integration are supported, runtime exceptions are not | Destack uses `Result`-first error handling |
| **Coercion hooks** | `valueOf`, `toString`, `Symbol.toPrimitive` | not used for implicit coercion | conversion should be explicit and typed |
| **Symbol magic** | `Symbol.hasInstance`, `Symbol.species`, `Symbol.isConcatSpreadable` | not supported | use typed protocols such as `iterator()` / `asyncIterator()` |
| **Callable `Symbol`** | `Symbol("name")` | not supported | use `Symbol.create("name")` or `Symbol.for("name")` |

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

TypeScript inherits its primitive types from JavaScript: `object`, `string`, `boolean`, `number`, `bigint`, and `symbol`, plus the `null` and `undefined` sentinels.
Destack provides a more complete primitive type system:

- precise numeric types beyond `number`, with variable-width signed and unsigned integers (`int8`, `uint32`, `int17`) as well as single and double precision floats (`float32`, `float64`)
- pointer-sized integers, i.e. integers as wide as the target pointer size, spelled `isize` and `usize`
- `int` and `uint` as aliases to `int64` and `uint64`
- `number` as an alias for `float`, and `float` as an alias to `float64`
- `char` as a single Unicode scalar value, distinct from `string`
- `unknown` as the explicit top type
- `never` as the explicit bottom type
- no `any`

Following the spirit of TypeScript's widening rules, numeric literals start as exact values and can flow into any numeric type that can represent them.
When no specific numeric context fits, the literals widen as usual to plain `number` (i.e. `float64`).

```ds
const id: uint64 = 12345;
7 satisfies uint3;
7 satisfies uint2; // error

const exact: int = 42;
exact satisfies int64;

const balance: float = 100.50;
balance satisfies float64;

const n: number = 1.0;
n satisfies float;

const initial: char = 'A';
const input: unknown = readInput();
```

### Intervals

Ranges in type position define an interval type over bounded sets like `int`, `bigint`, or `char`; basically, an interval type is a static subset of its scalar type.
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

### Newtypes

TypeScript is structurally typed: an interface is satisfied by any value matching its shape, regardless of whether it explicitly `implement`s it.
However, sometimes explicit nominality is helpful for correctness and expressiveness, and Destack adds `newtype` as the nominal counterpart to `type`.

For example, with plain `type`s and aliases, there is no actual protection against accidental assignment.
(The TS ecosystem commonly resorts to "branding" hacks to work around this limitation.)
```ts
type UserId = number;
0 satisfies number; // OK, TS is happy, but ouch

type OrderTag = string;
"invalid" satisfies OrderTag; // OK, TS still happy, also ouch
```

With explicit `newtype`, backing values do not satisfy the nominal type on their own:

```ds
newtype UserId = number;
const userId: UserId = 0; // error

newtype OrderTag = string;
const orderTag: OrderTag = "invalid"; // error
```

To construct a newtype value, use explicit `T(..)` call syntax:

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

Newtypes are representation-transparent to the compiler but opaque to the type system.
Construction and projection across the backing boundary are both explicit and zero-cost:

```ds
const id = UserId(1);
const raw = id as number;
```

### Newtype Interfaces

Newtype aliases add nominality to any type, and Destack also supports **nominal interfaces** using the `newtype` modifier on `interface` declarations.
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
Newtype interfaces are used for explicit behavioral traits like operator interfaces (e.g., `Add`, `Compare`), and for capability traits (e.g., `Send`, `Sync`, `Copy`, and `Clone`).

### Erasure

Structural and nominal `interface`s / `type`s are represented as transparent value constraints: any bare `T` of an `interface` or `type` becomes an implicit generic parameter in its declaration that is then substituted ("monomorphized") on application (similar to Rust's `impl T`).
That means the following interface-like declarations have same structure:

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

Transparent constraints like `type` and `interface` give the compiler a lot of optimization freedom in specialising methods and types.
For example, the following function declarations are representationally equivalent but specialise for concrete `Writer` implementations:

```ds
// use Writer as a regular parameter type, no explicit generics
function write(writer: Writer, bytes: [uint8]): Result<uint, Error> {
    writer.write(bytes)
}

// behaves exactly as if write had been written
function write<T: Writer>(writer: T, bytes: [uint8]): Result<uint, Error> {
    writer.write(bytes)
}
```

Nominal declarations like `newtype` aliases, however, define a new nominal value type whose representation is selected from its backing type expression.
Specifically, this means `newtype Shape = Rectangle | Circle` creates a concrete variant layout for `Shape`, while `type Shape = Rectangle | Circle` remains a transparent union constraint until some value or storage boundary asks for representation.

### Any

Interfaces being implicit static parameters is generally great for performance in a `type`-heavy language like TypeScript, and it works especially well because we always compile statically from source.
However, sometimes explicit _runtime_ indirection is desired, and Destack also provides an intrinsic `Any<T>` wrapper as the explicit erased runtime value satisfying some `T`:

```ds
// just like the function, this Logger is implicitly generic over Writer
struct Logger {
    writer: Writer;
}

// the Logger above is the same as Logger<T: Writer>
struct Logger<T: Writer> {
    writer: T;
}

// for fixed layout, use Any<Writer>
struct LoggerFor {
    writer: Any<Writer>;
}
```

In general, contract and transparent shapes give the checker room to specialize ordinary TypeScript-looking code, while value declarations, runtime joins, and `Any<T>` are the points where the program asks for a stable representation.

### Representation

Representation is the concrete storage and ABI shape selected for a representable type under the active target, the default representation being `@repr("destack")`.
The exact layout of a type can be configured via decorators that constrain its representation as needed, the conventions being very similar to Rust's:

| Decorator | Meaning |
| --- | --- |
| `@repr("destack")` | Use the native Destack representation. |
| `@repr("C")` | Use the active target's C ABI layout. |
| `@repr("transparent")` | Give a single-field declaration the same ABI representation as its field. |
| `@repr(T)` | Use primitive scalar `T` as an enum backing representation. |
| `@align(N)` | Raise the minimum aggregate alignment to `N`. |
| `@packed` / `@packed(N)` | Lower the maximum field alignment, with `@packed` equivalent to `@packed(1)`. |

```ds
@align(64)
struct CacheLine {
    value: uint64;
}

@repr("C")
@packed
struct WireHeader {
    tag: uint8;
    size: uint32;
}
```

Destack also supports querying parameters of the effective representation during compilation - available as a static term during inference - for conditional branching and storage:

| Intrinsic | Result |
| --- | --- |
| `sizeOf<T>()` | The byte size of `T`. |
| `alignOf<T>()` | The required alignment of `T`. |
| `strideOf<T>()` | The spacing between adjacent array elements of `T`. |
| `layoutOf<T>()` | The reflected `size`, `align`, `stride`, and shape for `T`. |

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

// extension may be in a different file or package alltogether
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
        ...
    }
}
```

The visibility of extension members follows from their placement:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension of int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils of Date { ... }`).

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
Using Destack's nominality via `newtype` and the builtin `Tagged` `derive`, TypeScript's well worn discriminated unions become even more ergonomic sum types:

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

### Classes

Classes follow the TypeScript-shaped model for managed objects with identity, except of course without a prototype chain or any dynamic class shenanigans.
Also, class fields require every instance field to be initialized by its declaration, a parameter property, or every constructor path.
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

Class methods are concrete by default and must be declared as `virtual` to enable _virtual_ dispatch of instances methods in subclasses.
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

Fixed arrays are homogeneous arrays whose length is statically known and part of the type.
They are inline value/layout types by default, and definitionally cannot grow.
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

user.profile.name = "Grace"; // error
user.tags[0] = "admin";      // error
```

As in TypeScript, `readonly` is a type-level access promise.
It does not freeze the runtime value.

### Generics

Destack supports classic TypeScript-shaped generics: inference, constraints, defaults, conditional types, mapped types, indexed access types, and the rest of the usual machinery.
The main addition is that generic parameters can also be _values_ that are then substituted into expressions _and_ are also available during inference.
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

Type inference is local and flows outward - we can "import" inference from other modules, but this only works one way; each module can infer static types and values from its own declarations and imports, and downstream modules can use what it exports.
Downstream uses do not feed back into upstream inference in any way.

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

### Static

Unlike TypeScript, Destack actually _compiles_, so we need to figure out during "compile time" the final type of each value and fill in values for all the known constants.
To do this, the "evaluation time" of the program is conceptually split into three successive worlds that only flow forward:

| World | Meaning | Example |
|-------|---------|---------|
| Static | types, values, and relations known while checking | `T`, `N`, `this.Width`, `T extends string? A : B` |
| Comptime | ordinary code explicitly evaluated by the compiler | `comptime factorial(10)` |
| Runtime | ordinary program execution | `readFile(path)`, `worker.postMessage(msg)` |

The statically known language forms known to inference are called **static terms**: static evaluation is done automatic during inference, and it is restricted to a small subset of the language (like TypeScript type operators), and it can _not_ execute `comptime <expr>` expressions.
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

Static terms are required wherever the language needs an inference-known answer: fixed array lengths, conditional types, associated members, static decorators, layout queries, and placement algebra.
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
At that point the `@if` guards become ordinary yes/no decisions and the concrete shape is known.
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
interface Iterator {
    type Item;

    next(): Option<this.Item>;
}

function collect<I: Iterator>(iter: I): I.Item[] {}
```

Associated types are type aliases scoped to some struct, class, or interface and can also reference the owner's generic parameters.

```ds
interface Allocator {
    type Pointer<T>;
    type Error;

    allocate<T>(count: usize): Result<this.Pointer<T>, this.Error>;
    free<T>(ptr: this.Pointer<T>): void;
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

Associated members participate in the same static evaluation world, and so associated members can express dependent types and values.

```ds
interface Matrix<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Bytes = [uint8; this.Width];
}
```

### Constraints

Sometimes defining the constraints and relations for type parameters can become unwieldy or outright impossible with only type annotations for each individual term.
Destack supports explicit `where` clauses to define additional cosntraints for complex types and signatures:

```ds
function merge<T: int, U>(): T where (
    U: Comparable<T>
) { }
```

### Reflection

TypeScript types are - by design - erased at runtime, which means we can't easily perform runtime type checks or any meaningful reflection.
Destack supports type reflection both at runtime and at compile time with `Type<T>` as a normalized view.
Any type expression can be turned into its reflected type with (implicit or explicit) casting to its `Type` representation:

```ds
struct User {
    name: string;
    age: uint;
}

let u: User = User { name: "Alice", age: 30 };

const UserType: Type<User> = User;
const UserType = Type.of<User>();
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
type InlineBytes<T> = [uint8; sizeOf<T>()];
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
By default, the captured environment is shared for the lexical scope, so multiple closures that share the same binding see the same environment, but this capture policy can be configured via the `@capture` decorator:

| Policy | Meaning |
| --- | --- |
| `"share"` | share the original binding through the managed lexical environment |
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

Closures with custom capture behavior must still follow general ownership rules, for example, if one closure moves a binding, later uses or captures of that binding are rejected.

### Continuations

Async functions and generators are closures that can pause and be resumed at a later point via stackful `Continuation`s, and the runtime just parks the live frame in a Worker-local "continuation handle".
`Promise`, `Generator`, and `AsyncGenerator` are "just" standard library types around this simpler `Continuation` primitive:

| Form | Meaning |
|------|---------|
| `ContinuationHandle` | Worker-local handle to a parked frame |
| `Promise<T>` | Worker-local async result object |
| `Generator<Y, R, N>` | Worker-local suspended generator |
| `AsyncGenerator<Y, R, N>` | Worker-local suspended async generator |
| produced `T` | value eventually produced by async code |

As in TypeScript, `await` and `yield` are the suspension points for the `Promise`s and `Generator` (and `AsyncGenerator`) coroutines where the entire stack up to that point is parked, and some other task is run.
In pure managed land, suspension works as before, and managed values can be stored in parked frames because it's all - well - managed.

```ds
type User = { name: string; };

async function read(user: User): Promise<string> {
    const name = user.name;
    await tick();
    return name; // valid as before
}
```

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
| Struct | `Point { x, y }` | destructure a nominal struct |
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

For exhaustive pattern matching, Destack supports the `match` expression:

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

Some patterns are irrefutable, which means they always match, and then we do not need any alternative branches, so no need for `_` or destructuring of known shapes.
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

### Guards

Guards are boolean expressions that can refine types, like the familiar `typeof value == "string"`, `"name" in value`, and `instanceof` checks:
 - `typeof` for primitive families (`"string"`, `"number"`, `"boolean"`, `"bigint"`, `"symbol"`, and `"undefined"`).
 - `"name" in value` for object types, and is quite imprecise.
 - `instanceof` for classes.

Destack adds an additional `value is T` check, which asks whether the current runtime representation of `value` carries the case or identity for `T`:

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

Like other type predicates, `value is T` returns `boolean` and narrows the branch:
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
        break ("done");
    }
    process(input);
};

let status = outer: loop {
    break outer: "done";
};
status satisfies "done";
```

### Using

Explicit resource management - `using` and `await using` - follows the [TC39 explicit resource management proposal](https://github.com/tc39/proposal-explicit-resource-management) with `using` / `await using` as explicit scoped cleanup, but of course using nominal interfaces instead of magic `Symbol` keys:
- `using` accepts `Dispose | null | undefined`.
- `await using` accepts `AsyncDispose | Dispose | null | undefined`, and falls back to synchronous disposal when the resource only implements `Dispose`.
- `null` and `undefined` are ignored, following the spec.

Resources are cleaned up at lexical scope exit in LIFO order, and `await using` runs async cleanup when required.
Cleanup - that is, the dispose function - runs when the scope exits for any reason: fallthrough, `return`, `break`, `continue`, or `?`.
The same using form also works in loop form, where it applies for every iteration, just like in the TC39 proposal.

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
Logical operators (`&&`, `||`, `??`), optional chaining, assignment, and strict identity (`===`, `!==`) are not (directly) overloadable, as usual.

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
| `==`, `!=` | `a == b` | `Equal<T>` or `PartialEqual<T>` |
| `<`, `<=`, `>`, `>=` | `a < b` | `Compare<T>` or `PartialCompare<T>` |
| `[]` | `a[i]` | `Index<I>` |
| `[] =` | `a[i] = v` | `IndexSet<I, V>` |
| `*` | `*a` | `ReadonlyDereference` |
| `* =` | `*a = v` | `Dereference` |

Dereference operators are a little different from the main "value-shaped" operators.
`ReadonlyDereference` and `Dereference` project one access form into another access form, preserving ownership, placement, access, and lifetimes.

```ds
struct Box<T> {
    ptr: ^T;
}

extension<T> of Box<T> implements Dereference {
    type ReadonlyOutput = &readonly T;
    type Output = &T;

    readonlyDereference(): this.ReadonlyOutput {
        &readonly *this.ptr
    }

    dereference(): this.Output {
        &*this.ptr
    }
}
```

Explicit `*box` uses `ReadonlyDereference`, while assignment through `*box` needs mutable `Dereference`.
Member lookup and method calls may autoderef through `ReadonlyDereference` / `Dereference`, but only after checking the wrapper's own members first.
Autoderef does not make `Box<T>` generally assignable to `T`; it is just member lookup ergonomics for smart pointers and view-like wrappers.

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

Ranges work in patterns and subscripts exactly like one would expect from other languages::

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

As discussed in Types, structural interfaces keep normal TypeScript shape checking and mostly work exactly as expected: bare structural interface types as transparent constraints, so `point: PointLike` behaves like an implicit `T: PointLike` parameter and is specialized for the concrete argument type.
Because structural interfaces are satisfied by shape, writing `implements` on one is only an explicit declaration-site check.
Erased interface values are spelled explicitly with `Any<T>`.

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

Index signatures also work, and dispatch through the `Index` / `IndexSet` operator interfaces.
`Record<string, T>` can satisfy this with map lookup, and custom types can satisfy it by implementing the corresponding index protocols (trivially satisfiable by including a builtin map-able type).

```ds
interface Bag<T> {
    [key: string]: T | undefined;
}

const counts: Bag<int32> = { apples: 3, oranges: 2 };

function read<T>(bag: Bag<T>, key: string): T | undefined {
    bag[key]
}

read(counts, "apples") satisfies int32 | undefined;
```

#### Unions

Union receivers - the member being dispatched on - are resolved per variant, and every variant must expose that member.
If all variants resolve to the same implementation, the call is static; otherwise the result type is the union of the selected return types and the compiler emits dynamic checks to dispatch on the right member at runtime.

```ds
struct TcpStream {
    write(chunk: [byte]): Result<usize, IOError> {}
}

struct MemoryBuffer {
    write(chunk: [byte]): Result<usize, never> {}
}

function writeAll(sink: TcpStream | MemoryBuffer, chunk: [byte]) {
    const written = sink.write(chunk);
    written satisfies Result<usize, IOError> | Result<usize, never>;
}
```

### Errors

Banishing exceptions is Destack's biggest divergence from TypeScript: Destack uses **Result-first error handling** exclusively, and throwing exceptions is not allowed in any native Destack code.
Recoverable errors use `Result<T, E>`, integrate with `try` / `catch`, and can be opened with `?`, `??`, and postfix `!`.
(JavaScript exceptions remain valid _syntax_ because we need to integrate with JS targets directly, but in regular userland, exceptions are basically forbidden.)

#### Error

Like in Rust, types that want to be handled as general errors explicitly implement the nominal `Error` interface:

```ds
newtype interface Error {
    display(): string;

    source(): Any<Error> | undefined {
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
// -> failure: E | null | undefined
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

The try-coalesce operator `??` accepts the same shape locally "within" the expression giiven a direct fallback instead of letting it bubble up.
The result of `Result<T, E> ?? F` is the non-nullish opened success type joined with the fallback type `T | F`.

```ds
declare const defaultConfig: Config;

declare function loadConfig(): Result<Config, IOError> | null;
const a = loadConfig() ?? defaultConfig;
a satisfies Config;

declare function loadMaybeConfig(): Result<Config | null | undefined, IOError | null> | undefined;
const b = loadMaybeConfig() ?? defaultConfig;
b satisfies Config;
```

All unwrap operators only unwrap _one_ layer of `Try`, so nested `Try` values inside the success type also stay wrapped at the inner layer:

```ds
declare function loadNested(): Result<Result<Config, ParseError>, IOError>;

const c = loadNested() ?? defaultConfig;
c satisfies Result<Config, ParseError> | Config;
```

Postfix `!` is the "must" forced unwrap form: it opens the same outer nullish and single `Try` layer, but traps instead of propagating or falling back when the value is absent or failed:

```ds
const config = loadConfig()!;
config satisfies Config;
```

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

### Trees (TSX)

TypeScript XML (`.tsx`) is a great way of writing UI-shaped code and has even seen some adoption for other tree-shaped data structures as well.
Destack (`.ds`) natively supports `.tsx` like constructs with the same rules:

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

Unlike in TypeScript, in Destack types can participate in custom tree tag behavior by implementing the `TreeTag` interface, and custom intrinsic types (lowercase tags like `<div>`) are programmable via `TreeTagBuilder`.
Essentially, `TreeTag` generalises `jsxFactory` and `TreeTagBuilder` generalises `jsxFragmentFactory`:
 - Uppercase or qualified tags resolve as value tags through normal value lookup and the `TreeTag` interface.
 - Lowercase unqualified tags resolve as intrinsic tags through the active `TreeTagBuilder`.

The active `TreeTagBuilder` comes from the compiler / target / profile options by default, but can be locally overridden with module metadata.

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

#### Taint

Destack systematises the idea of "taints", "source", and "unsafe" modifiers on expressions and declarations using its annotation system:
 - `@taint("tag")` marks a value as carrying some domain, `@untaint("tag")` unmarks it as no longer carrying that domain.
 - `@source("domain")` marks an operation that produces some domain, `@sink("domain")` marks an operation that receives some domain.
 - `@unsafe` marks an operation that is unsafe to call, `@safe` marks an operation that is safe to call.

```ds
@unsafe
declare function read<T>(pointer: *T): ^T;

@safe
function get<T>(items: Slice<T>, index: usize): T {
    if index >= items.length {
        panic("index out of bounds");
    }

    return items.unsafeGet(index);
}
```

#### Derive

Similar to Rust, Destack supports `@derive` providers for extending certain declarations at compile time.
Unlike in Rust, a derive provider is just a nominal decorator that happens to implement the `Macro<Target>` interface, and `derive`-like macros do not need to be implemented in a different package (or "crate") or in any special syntax.

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

At the library level, a `derive` provider is just a nominal provider value that implements `Macro`, with some additional instrumentation.
Really, `derive` is basically a convenience wrapper for applying multiple `Macro` providers to a single target in a well known way.

Auto-derive uses the same providers.
Configured auto-derive providers run over every nominal declaration (`class`, `struct`, `enum`, and `newtype`), and providers that do not apply simply emit nothing.
Explicit `@derive(...)` is stricter: if a provider is written directly on a declaration and cannot apply, it should report an error.

#### Static If

Destack also supports a special `@if` decorator that gates the inclusion of certain nodes based on a static term.
When the condition is false, the annotated item is (in effect) removed from the instantiated shape.

```ds
interface FileSystem {
    open(path: string): Result<File, IOError>;

    @if(import.meta.platform != "windows")
    chmod(path: string, mode: uint16): Result<void, IOError>;

    @if(import.meta.platform == "windows")
    setAttributes(path: string, attrs: WindowsFileAttributes): Result<void, IOError>;
}
```

`@if` works on module declarations, class and struct members, interface members, enum fields, and even statements:

```ds
struct Buffer<T, comptime Mode: "inline" | "external"> {
    @if(Mode == "inline")
    index: InlineIndex;

    @if(Mode == "external")
    index: ExternalIndex;

    data: T[];
}
```

### Module

Destack modules can contain (up to) one static `module { ... }` declaration block for source-level configuration that needs to be specific to a module.
Usually, we would configure via the compiler / target / profile options, but sometimes it's helpful to override some of these options locally:

```ds
import { HtmlTree } from "destack:ui/html";
import { Clone, Debug } from "destack:decorator";

@noHeap
module {
    const tree = HtmlTree;
    const derive = [Debug, Clone];
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

### Globals

TypeScript supports ambient global typings, which were designed for typing the "magic" global objects provided by embedders, but it has no way to contribute _value_ globals in userland.
Destack supports "real" value `global { ... }` declarations that can then be automatically included everywhere by (explicit) reference in the compiler / target configuration.

```ds
// browser-globals.ds
global { // just omit the `declare`!
    const window: Window = runtime.browser.window();
    const document: Document = runtime.browser.document();
}
```

Because these globals are real values, duplicate visible global value names are errors.

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

It is important to note that functions do not need to declare themselves as either "comptime" or "runtime": the same function can run at compile time when all inputs are static, and at runtime when some input is only known at runtime:

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

Comptime blocks can also appear as members on object-like types for static checks.
They run in the static environment of the declaration or instantiation where they appear post-inference, and can access the same static terms as `@if`.

```ds
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0 && size <= 65536);
    }
    data: [uint8; size],
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
        assert isPowerOfTwo(Width);
    }
}
```

Of course, comptime results must also be lowerable into the target artifact.
Plain data such as numbers, strings, arrays, tuples, objects, structs, and enums are all fine, but dynamic runtime resources like pointers and handles and such don't work because we can't meaningfully serialize them.

#### Dynamic Code

Generating and evaluating arbitrary code is supported via `eval` at _compile-time_ by passing the static term of a string:

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

As explained in the annotations and decorator piece, `memoize` by itself is just an inert annotation and it only receives behavior by implementing `Macro`.
The `Macro` system is based on three rules:
 1. Macro expansion is recursive and runs until there is nothing more to expand (or we encounter an error).
 2. Macros run in two phases during compilation: `expand` may contribute new symbols before final inference, while `materialize` fills in implementation details with full type information.
 3. Macros interact with their containing module through phase-specific context methods (`resolve`, `ensureImport`, `add`, `ensureDeclaration`, `addChild`, `replaceTarget`, `renameTarget`, `removeTarget`).
 4. Macro invocations are exclusively triggered by decorators implementing `Macro` and by the (sparingly used) auto-derive providers.

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

Destack supports explicit, optional type modifiers for controlling memory _ownership_ and _placement_:
- **Ownership** - who "owns" the value: managed (`T`), owned (`^T`), borrowed (`&T`, `&readonly T`, `&exclusive T`), or raw (`*T`).
- **Placement** - where the value is located: ambient by default, explicitly `local` to one Worker, `shared` across Workers (or `static` for constant and `frame` for activation frames).

The two axes of ownership and placement compose and commute freely, e.g. `shared ^T` and `^shared T` both mean an owned handle to a value in shared space, and `shared &T` is a borrow of a shared value.
Importantly, the plain old `T` still behaves as the type's default representation, exactly like we're used to from TypeScript: value types are values, object types are managed references, both can be aliased and mutated freely (locally).

### Ownership

Ownership determines who keeps a value alive, who is allowed to mutate it, and when and how it is eventually freed.
The notion of ownership is old but most commonly associated with Rust's explicit ownership system, and that is also the system most similar to Destack's implementation with a few tweaks.
Really, "ownership" just means that _values_ have an owner and certain rules apply to how we can pass and store values to certain places, depending on which level of mutability and ownership they need.

The usual explanation of "ownership" sounds more complex than it is, especially to developers used to "managed" languages, and _especially_ because Rust tradition (deliberately) unifies "liveness", "exclusivity" and "mutability".
Unlike Rust, Destack we support _both_ multiple mutable borrows (`&T`) and exclusive mutable borrows (`&exclusive T`):

| Form | Meaning | Mutable? | Exclusive? |
|------|---------|----------|------------|
| `T` | normal managed/default value | yes | no |
| `^T` | owned value | yes | yes (single owner) |
| `&T` | borrowed access | yes | no |
| `&readonly T` | readonly borrowed access | no | no |
| `&exclusive T` | exclusive borrowed access | yes | yes |
| `*T` | raw pointer | yes (unchecked) | no (unchecked) |

The ownership and borrow checking logic follow from the two rules that borrows must always be valid, and that exclusive borrows must indeed be exclusive.

```ds
let a: User = new User();
let b: ^User = new User();
let c: &User = &a;
let d: &readonly User = &readonly a; // OK: &User is *not* exclusive
let e: &exclusive User = &exclusive a; // ERROR: &exclusive User *is* exclusive
let e: *User = &a;
```

Each ownership form also has a corresponding normalized representation in our little ["type algebra"](###algebra), which means we get to do regular TypeScript-style type space logic, conditionals and remapping (including for lifetimes!).

### Space

Space determines where some value is actually located in memory, and since Destack follows web and JS/TS convention, we use the `Worker`-local heap as the default main memory space.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space, and user and library code can almost always just pretend spaces don't even exist.

```ds
struct Request<T> {
    header: Header,
    body: T,
}

let localRequest: Request<Body>;          // ambient, default -> Request is worker-local heap
let sharedRequest: shared Request<Body>;  // explicit, shared -> Request is shared heap
```

Memory placement is contextual: all types are "ambient" by default, i.e., they come with no inherent placement, and types are only placed wherever their parent is placed until someone either specifies placement explicitly (e.g., `local T`, `shared T`) or we reach the top, which - as established - is `local` to the Worker's own local heap by default.
This "ambient placement" rule is also why we distinguish `Place` from `Space`: `Space` is concrete, while `Place` may also be `"ambient"`.

#### Shared Space

In TypeScript tradition, each "worker" (in the spec also "Agent") has its own isolated local heap that cannot be touched by other works.
For sharing memory across workers, TypeScript has the `SharedArrayBuffer` concept and plain old message passing with `postMessage`.
That works, but is architecturally limited and makes it complex to implement more sophisticated parallelism patterns.

Destack supports an explicit, fully featured _shared_ memory space that is visible to all `Worker`s in the same `Runtime` via regular references and objects.
Basically, `shared T` is the typed, generalized version of the `SharedArrayBuffer` idea with the full type system and object graphs at our disposal with the simple rule that local may point into shared, but shared must not point into local memory.

It's important to note that _by itself_ shared placement, like any space placement, is **not** a synchronization primitive of and does **not** imply atomic access, locking, `Sync`, or anything like it.
That is by design; it's up to userland libraries to require capabilities like `Send` and `Sync` for APIs that transfer or publish values for correctness, but `shared` itself is really only about placement.

#### Static Space

Because Destack inherits the JS/TS Worker model for isolation, module-scoped constants are owned by each _Worker_ and are not actually process-global as they would be in most other languages.
For genuinely _shared_ process-global state, the binding _itself_ can be declared as `shared`.

| Form | Binding place | Value place | Meaning |
|------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

Note that making the binding itself as `shared` also types the value as `shared` (as it is illegal to point from shared storage into local storage anyway, this is convenient).
There is no `local const` form because ordinary module bindings are already Worker-local.

### Allocator

The primary way to construct new values is `new`, which initializes a `T` and produces the ownership form required by the _destination_ type - `new` is really just an initializer that calls the type's constructor.
The required destination type decides whether that is managed storage, owned storage, inline frame storage, shared storage, or some lower-level allocation form.

```ds
let a: User = new User();   // managed
let b: ^User = new User();  // owned
```

Of course, Destack also supports direct allocation control via direct access to the underlying `Allocator`:

```ds
let allocator = defaultAllocator<"shared">();
let layout = AllocationLayout { size: 4096, align: 64 };
let page = allocator.allocate(layout)?;

page satisfies Allocation<"shared">;
```

Memory-sensitive code can also opt out of managed allocation locally:

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

### Conversions

The rules for converting references follow from the three basic memory rules established above:
- References must always be valid (the referent must never be deallocated while the reference is live),
- Exclusive references must be truly exclusive (no possibly overlapping loan is live).
- Shared memory must not point into local memory (directly or indirectly).

| From | To | Allow | Explanation |
|------|----|-------|-------------|
| local managed `T` | `&T` / `&readonly T` | yes | while the source place stays live and the borrow does not cross suspension |
| local managed `T` | `&exclusive T` | yes | while no overlapping loan is live and the borrow does not cross suspension |
| shared managed `T` | `&T` / `&readonly T` | yes | shared-safe APIs remain responsible for synchronization and data-race safety |
| shared managed `T` | `&exclusive T` | no | a shared managed handle cannot prove uniqueness |
| owned `^T` | `&T` / `&readonly T` / `&exclusive T` | yes | while the owned source stays live and the requested loan rules hold |
| shared owned `shared ^T` | `shared &T` / `shared &readonly T` / `shared &exclusive T` | yes | owned storage remains unique even when placed in shared space |
| `&T` | `&readonly T` | yes | readonly reborrow |
| `&exclusive T` | `&T` / `&readonly T` | yes | temporary reborrow that suspends the exclusive loan |
| borrowed parameter | borrowed access across suspension | source-checked | the caller must prove the source is owned or static |
| `T` | `^T` | no | managed ownership does not become unique ownership |
| `&T` | `T` / `^T` | no | borrowed access does not own the value |
| `*T` | `&T` / `&readonly T` / `&exclusive T` | yes | explicit unsafe reborrow |
| `T` / `&T` / `^T` | `*T` | yes | explicit raw pointer conversion |

The default type for a borrow is `&T`, and typing it as `*T` produces a raw pointer instead:

```ds
let user = new User();
let userBorrow: &User = &user;
let userPointer: *User = &user;
```

As in Rust, just converting a borrow `&T` to a raw pointer `*T` by itself is perfectly safe; only dereferencing and manipulating raw pointers becomes `@unsafe`.

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

The entirety of borrow checking behavior - ensuring that a borrow to some reference remains valid - follows from two simple rules:
1. a place cannot move or drop while an overlapping borrow is live;
2. a borrow cannot outlive the owner or access path it came from.

Ownership is applied individually to each "access path", so disjoint fields can be borrowed independently when the compiler can prove they do not overlap.

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

### Lifetimes

The relationship between whoever owns the memory (managed or explicitly owned) and those who want to reference (borrow) is specified via Rust-like lifetimes, implemented as `<L: Lifetime>` and `Borrowed<T, L>` generics.
Destack's lifetimes are conceptually very similar to Rust, but as the spelling implies, `<L: Lifetime>`s are full generics available to regular TypeScript-style type algebra _and inference including flow typing and narrowing_, which means we very rarely need to spell them out.

```ds
function read(user: &User): &string {
    return &user.name;
}

function first<T>(items: &[T]): &T {
    return &items[0];
}

struct View<T> {
    items: &[T];
}
```

Because lifetimes are part of type inference, and type inference also analyses method bodies, lifetime inference also derives from function bodies:

```ds
function first(a: &Node, b: &Node): &Node {
    return a;
}

function choose(a: &Node, b: &Node, flag: boolean): &Node {
    return flag ? a : b;
}
```

It follows that declaration-only APIs must spell out their lifetime requirements explicitly when borrows may be ambiguous.

```ds
declare function only(value: &Node): &Node;

declare function choose<L: Lifetime>(
    a: Borrowed<Node, L>,
    b: Borrowed<Node, L>,
): Borrowed<Node, L>;
```

Stored borrowed fields use the same idea and get one hidden lifetime parameter, shared by all elided borrowed fields in the declaration:

```ds
struct EngineView {
    engine: &Engine;
}

struct WorldView {
    engine: &Engine;
    assets: &AssetStore;
}
```

Taken together, these mechanisms of elision and inference for lifetimes allow us to implement the vast majority of low level ownership patterns without having to specify lifetimes to the compiler explicitly.

### Suspension

As discussed in [Continuations](###Continuations), values and objects in "managed land" work exactly like in regular TypeScript and can be referenced and mutated freely wherever.
When _borrowing_ across suspension points (like `yield` or `await`), the compiler must still guarantee that the core safety rules are upheld.
And because multiple continuations may be active concurrently (even if not in parallel), borrows that must be owned by the current frame to cross suspension points - in other words, borrows coming from managed values must _not_ cross suspension points.

It follows that frame-owned values _can_ be borrowed across suspension points:

```ds
async function read(user: ^User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

Borrowed access is also valid across suspension points _if_ it originates in an owned or static value:

```ds
async function read(user: &User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

But borrowed access to managed values can _not_ cross suspension (even non-exclusive):

```ds
async function read(user: User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

### Drop

Whenever the lifetime of a value ends and it is deallocated, Destack supports running a `Drop` finalizer, similar to Rust's `Drop`.
This happens when the compiler inserts a drop for an owned local after its last use, when an owned field is being destroyed, and when the runtime reclaims an unreachable managed allocation.

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

Unlike Rust and C++ RAII, Destack models `Drop` and `Dispose` / `AsyncDispose` separately: `Drop` follows lifetime, while `using` is lexical.
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

### Unsafe

Safe Destack code can create and carry raw pointers, because there is nothing directly unsafe about just looking at pointers.
Raw pointers are inert: they do not keep storage alive, do not participate in borrow checking, and do not prove exclusivity.

Converting a borrow to a raw pointer is still safe because it does not touch the pointed-to memory:

```ds
let user = new User();

let borrow: &User = &user;
let pointer: *User = borrow; // ok: this only creates a raw pointer value
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
| raw allocation lifecycle | `allocator.allocate(layout)`, `allocator.deallocate(allocation)` | allocate yes, free no | allocation returns an inert token; free must match allocator and layout |

The compiler rejects unsafe operations, like raw pointer dereferencing, outside explicit `@unsafe` / `@safe` contexts.

### Algebra

Destack's "memory algebra" is a fancy way of saying that the axes of ownership, access, lifetime, and placement are just types that we can do TypeScript-style algebra and inference with.
Code can inspect a type's memory form and build a derived form because all the qualified surface forms like `readonly T`, `^T`, `&T`, `*T`, `local T`, and `shared T` correspond to builtin intrinsic types:

```ds
/// Automatically managed T, owned by the runtime.
newtype Managed<T> = intrinsic;
/// Owned T (`^T`).
newtype Owned<T> = intrinsic;
/// Borrowed T (`&T`).
newtype Borrowed<T, L: Lifetime, A: Access = "mutable"> = intrinsic;
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
    lock: SharedLock;

    value: T;
}

declare const localBuffer: Buffer<string>;
declare const sharedBuffer: shared Buffer<string>;

sharedBuffer.lock satisfies SharedLock;
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

Like other similar languages, Destack encodes synchronisation and memory primitives as (newtype) interfaces like `Copy`, `Clone`, `Send`, and `Sync`
(As said, these are separate from and orthogonal to ownership and placement.)

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |
| `Send` | Value can cross a Worker boundary. |
| `Sync` | References to shared values can be used concurrently through the type's own API. |

It should be noted again that `shared T` means `T` lives in shared space; it does _not_ make `T` automatically `Sync` by itself.
Userland APIs such as channels, Worker pools, atomics, locks, and actors can require `Send` or `Sync` when they need those stronger guarantees, very similar to Rust or even Swift.

### Synchronisation

The standard library provides the usual memory and synchronisation primitives on top of this unified memory system.
The full details are documented in the library, but the basics should be familiar to anyone with a systems-level background.

| Primitive | Contract |
|-----------|----------|
| `Box<T>` | unique heap ownership for `Owned<T>`, with deterministic drop when `T: Drop` |
| `Rc<T>` | local shared ownership of `Owned<T>`, non-atomic refcount, not transferable across Workers |
| `Arc<T>` | shared ownership of `Owned<T>`, atomic refcount, transferable when `T` satisfies the required `Send` / `Sync` bounds |
| `Cell<T>` | local interior mutation by value, for small `Copy`-like state |
| `RefCell<T>` | local runtime borrow checking for cases static borrowing cannot express cleanly |
| `Atomic<T>` | lock-free scalar storage with explicit ordering and scope |
| `AsyncLock<T>` | local mutual exclusion that suspends the current async task, not the Worker |
| `SharedLock<T>` | shared mutual exclusion backed by atomics and runtime wait/wake support |

Because ownership and placement are part of our type system, and we can query and gate based on contextual type information using regular TypeScript algebra, we have a lot of flexibility and gain some nice ergonomics.
For example, we can provide ergonomic context-aware aliases that conform to the way they are used:

```ds
type Ref<T> =
    PlaceIn<T, "local"> extends "shared" ? Arc<T> : Rc<T>;

type Lock<T> =
    PlaceIn<T, "local"> extends "shared" ? SharedLock<T> : AsyncLock<T>;
```

# Runtime

The runtime is where code actually _runs_, and it's where pure computation touches the real world via our well defined `host` bindings.
This is nice because it means we get to capture and analyse all effects through a relatively thin well known boundary, which enables great observability and debugging.
And because Destack is a fully integrated stack, the runtime has been co-designed as part of the entire language toolchain and the standard library takes advantage of this

## Modules

Like many JS/TS-adjacent runtimes, Destack supports importing additional file types beyond code modules.

### Conditions

Conditions generalise the idea behind `module.tests.ds` and `[cfg(attr)]`-style feature gating into a flexible "condition system" for, well, conditionally including sources and parts of sources into some builds (but not others).
We recognize `conditions` of `mode`, `feature`, `role`, and a general `tag`, in addition to all the usual target gates (e.g. `host`, `runtime`, `target`, `platform`):

```json:destack.json
{
    "conditions": {
        "modes": {
            "test": {},
            "dev": {},
            "prod": {},
            "preview": { "extends": "dev" }
        },
        "features": {
            "checkout": {},
            "rendererv2": {}
        },
        "roles": {
            "server": {},
            "client": {}
        },
        "aliases": {
            "browser": { "host": "browser" }
        }
    },
    "compiler": {
        "modes": ["preview"]
    }
}
```


The active conditions are available within code via `import.meta.<condition>` (like `import.meta.roles`) as usual for in-code dynamic gating:

```ds
function getRenderer(): Renderer {
    if (import.meta.features.includes("rendererv2")) {
        return new RendererV2();
    } else {
        return new RendererV1();
    }
}
```

On the import side, Destack also generalises `<module>.<alias>.ds` to support conditional inclusion of files (appended to the `module.ds` file itself) based on the active conditions via `aliases`.
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

Chained like `user.test.browser.ds` behave as "and" gates on all conditions, that is, `user.test.browser.ds` is included only when both `test` mode and the `browser` alias match.

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
| `import.meta.runtime` | semantic runtime | `Runtime` | `"destack"`, `"js"` |
| `import.meta.modes` | active source graph modes | `readonly Mode[]` | `["test"]`, `["dev", "lint"]` |
| `import.meta.roles` | active source graph roles | `readonly Role[]` | `["server"]`, `["client"]` |
| `import.meta.features` | active source graph features | `readonly Feature[]` | `["checkout"]`, `["renderer"]` |
| `import.meta.tags` | active source graph tags | `readonly Tag[]` | `["preview"]`, `["internal"]` |
| `import.meta.debug` | `debug` mode shorthand | `boolean` | `true`, `false` |
| `import.meta.dev` | `dev` mode shorthand | `boolean` | `true`, `false` |
| `import.meta.prod` | `prod` mode shorthand | `boolean` | `true`, `false` |
| `import.meta.test` | `test` mode shorthand | `boolean` | `true`, `false` |
| `import.meta.bench` | `bench` mode shorthand | `boolean` | `true`, `false` |
| `import.meta.lint` | `lint` mode shorthand | `boolean` | `true`, `false` |
| `import.meta.env` | configured build environment | `{ readonly [key: string]: string | boolean | number }` | `{ NODE_ENV: "production", FEATURE_X: true }` |
| `import.meta.tree` | current module tree tag builder | `TreeTagBuilder | undefined` | `HtmlTree` |
| `import.meta.derive` | current module auto derive providers | `readonly Macro<unknown>[]` | `[Clone, Debug]` |
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
