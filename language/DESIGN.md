# "Language"

The Destack language (`.ds`) and toolchain, colloquially "TypeScript++", are a superset of "strict modern" TypeScript with support for `.ts` and `.tsx` files, native AOT compilation and a fully integrated language toolchain, _and_ it can also compile nicely to standard JS/TS targets.
We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated stack (Destack), and thus by "language" ("TypeScript++") we mean much more than "just" a syntax form: a language, a runtime, a toolchain, plugins, and ultimately, a way of programming.

## "TypeScript++"

We're very early in software, and we're still figuring out how to build optimal, correct, and integrated software systems.
Over 50 years, we have grown more and more layers of software sediment and need ever _more_ tools to get any code out the door, and yet confidence and performance have plummeted.

We believe the best possible stack is the most integrated one, and it must truly span the entire lifecycle: the language itself, the toolchain with linters and formatters, a VM, compiler, runtime, and basically anything that touches the code.
Only TypeScript is seriously close to being a universal software foundation, because it runs directly on the web, and the web is the most ubiquitous application platform.
The TypeScript ecosystem has good - if not perfect - answers to all modern software needs, from great developer tools to rich interactive frontends to quite _decent_ and performant backends.

If you remove all the JS baggage and dynamic prototype mess, modern TypeScript is surprisingly close to a fully AOT-compilable language (and most browsers retrofit compilation internally already based on these assumptions).
Embracing TypeScript and "the web ecosystem" lets us build a new toolchain that truly covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be completely free of JS overhead and (some) historic baggage.

## Compatibility

**Destack is a superset of the "modern strict" subset of TypeScript**.
The intended use case for Destack is making TS-shaped code optimal and correct, which requires cutting all accumulated dynamic magic that might smuggle in ambiguity or unsoundness at runtime.
Accordingly, Destack excludes legacy syntax and all sorts of dynamic shapes and protocols that are not statically sound, and also removes a few rarely used footguns.

### Expressions

Some JS/TS syntax and legacy behavior is either ambiguous, obsolete, or just not worth carrying forward:

- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript.
  All non-strict ("sloppy mode") behaviors like duplicate function declarations, `arguments` magic, `caller` / `callee`, or `yield` as an identifier are not supported.
- **Type-only imports and exports**: `.ds` accepts `import type` and `export type` for TypeScript familiarity, but they behave the same.
- **CommonJS**: Destack source does not support `require`, `module.exports`, mutable `exports`, require-cache monkeypatching, `export =`, or `import x = require("x")`.
- **Ambiguous generic arrow**: `<T>() => ...` is ambiguous in `.tsx` because it might be a TSX tree, and `.ds` inherits this since we support TSX syntax natively.
  To disambiguate, use `<T,>() => ...`.
- **Sequence expressions**: `(A, B, C)` is - confusingly - a "sequence expression" in JS, which nobody ever really types out by hand, and `.ds` instead claims `(A, B, C)` for explicit tuples.
- **Single-quoted literals**: In `'A'` is a `char`, not a `string`.
  Use double-quoted string literals for text, while `.ts` and `.tsx` keep TypeScript's ordinary single-quoted strings.
- **Loose equality coercion**: Object coercion through `==` and `!=` is not allowed.
- **XML namespace resolution**: Destack does not implement XML `xmlns` namespace binding semantics.
  Namespaced tree tags like `<svg:path />` are treated as intrinsic string tag names (`"svg:path"`).
- **Dynamic module loading**: Runtime `import(expr)` is not general module loading in source code.
  JS output may still use dynamic imports for chunk loading when the target requires it.
- **Dynamic code generation**: Dynamic _runtime_ `eval` / `new Function` / class generation are in conflict with a strict AOT model and unsupported, **but** Destack supports explicit `comptime eval` / `new Function`.
- **Exceptions**: Destack does not support _executing_ exceptions in any way - `.ds` still supports `throw`, `try`, `catch`, and `finally` syntax for JS-target compatibility, and try-catch-finally even work with our `Try` / `Result` types, but that's it. No runtime exceptions of any kind.

### Types

Dynamic shapes and unsound types are incompatible with a strict sound compilation model:

- **Flow and JSDoc _typing_**: We support TypeScript only.
  Flow syntax and special JSDoc type analysis are ignored / rejected where they are not valid TS/TS++.
- **Thenables**: `await` only works on the well known `Promise<T>`, not "anything with `.then`".
- **`any`**: `.ds` uses `unknown` as the top type which must be explicitly cast before using it.
  TypeScript `any` is rejected because it makes arbitrary property access, calls, and assignments appear valid without proof.
- **Definite assignment assertions**: `let x!: T` and `field!: T` are rejected in `.ds`.
  Locals and fields must be actually initialized before use, whether directly with an initializer or just with control flow.
- **Declaration expressions**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with proper AOT compilation.
- **Prototype objects**: `.prototype`, `.__proto__`, `.constructor`, `Object.getPrototypeOf`, `Object.setPrototypeOf`, and `Object.create(proto)` all rely on the prototype-based object model and are not supported.
- **Shape mutation**: `delete`, `Object.defineProperty`, `Object.defineProperties`, `Reflect.defineProperty`, `Reflect.deleteProperty`, and shape-changing `Object.assign` are forbidden.
- **Metaobject dispatch**: `Proxy` and most `Reflect.*` APIs exist to intercept or emulate dynamic object behavior, so they are also unsupported.
- **Class index signatures**: TypeScript permits structural index signatures inside classes, but Destack classes have fixed declared members.
  Put index signatures on structural object types or interfaces instead.
- **Array holes**: Destack does not permit "holes" in arrays like `[1,,3]`.
- **Circular inference**: Destack does not support circular inference _across_ modules.
  Modules may export types they can establish from local declarations _and_ imports, and downstream modules may build on those exports, but downstream uses do not refine upstream declarations.
- **Enum coercion**: `enum Level { A = 1, B = 2, C = 3 }` is _just_ an alias in TypeScript, but Destack does _not_ coerce `Level.A` to `number` without an explicit cast for better soundness.
- **Coercion hooks**: `valueOf`, `toString`, and `Symbol.toPrimitive` do not participate in implicit object coercion.
- **Symbol magic**: `Symbol.hasInstance`, `Symbol.species`, `Symbol.isConcatSpreadable`, and other such hooks are not supported.
  Use typed `iterator()` / `asyncIterator()` protocols.

# Language

"TypeScript++" is a superset of "strict modern" TypeScript, which essentially means that existing TypeScript (and TSX!) _just works_ **if** it follows our strict TypeScript-based type system.
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

Newtype aliases add nominality to any type, and Destack thus also supports **nominal interfaces** using the `newtype` modifier on `interface` declarations as a convenience.
This makes newtype interfaces behave essentially like traits in other languages.

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

### Any

All type constraints - structural and nominal `interface`s, `type`s, whatever - are represented internally as _static_ value constraints by default, and thus any bare `T` of an `interface` or `type` becomes an implicit generic parameter that is monomorphized on application (similar to Rust's `impl T`).
When explicit _runtime_ indirection is desired, Destack also provides an intrinsic `Any<T>` wrapper as the explicit erased runtime value satisfying some `T`.

For example, consider the following representation-equivalent `Writer` interfaces:

```ds
// regular interface
interface Writer {
    write(bytes: [uint8]): Result<uint, Error>;
}

// nominal interface
newtype interface Writer {
    write(bytes: [uint8]): Result<uint, Error>;
}

// regular type
type Writer = {
    write(bytes: [uint8]): Result<uint, Error>;
}

// nominal type
newtype Writer = {
    write(bytes: [uint8]): Result<uint, Error>;
}
```

The following two functions are conceptually equivalent:

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

This is generally great for performance in a `type`-heavy language like TypeScript, and it works extra well because we always compile statically from source.
Sometimes we still want to trade runtime overhead for code size or just have a fixed layout for some other reason:

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

In general `Any` behaves just like one would expect, and `Any<type {}>` is the empty erased type matching any value, akin to Go's `interface{}`.

### Extensions

It is sometimes convenient to attach additional logic and data to the (nominal identity of) a type.
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

extension of Vector2 {
    static ZERO = new Vector2(0.0, 0.0);

    magnitude(): float32 {
        return (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

Extensions can be added to any **nominal type**, including `struct`, `class`, `enum`, and `newtype`, whether defined locally or in a foreign / imported module.
Plain type aliases (`type X = ...`) and structural types (`{ x: number }`) cannot receive extensions because it would be unclear when they should apply.

Extensions can also be named for explicit exports and imports:

```ds
import { User } from "@/model/user";

export extension UserUtils of User {
    validate(): bool {
        ...
    }
}
```

The visibility of extension members is straightforward:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension of int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils of Date { ... }`).

### Enums

Enums are nominal aliases to a set of constants, just like in TypeScript, except that Destack's enums do not implicitly cast to their backing type and explicit conversions are required for the backing value type.
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
Using Destack's nominality `newtype` and the builtin `Tagged` `derive`, the well known discriminated unions become pretty presentable sum types:

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

extension of Shape {
    static DEFAULT = Shape.Rectangle({ width: 10, height: 20 });
    
    variant() {
        match (this) {
            Shape.Rectangle { ... } => "rectangle"
            Shape.Circle { ... } => "circle"
        }
    }
}

// create values of tagged newtype unions with <Type>.<Variant>
const rectangle = Shape.Rectangle({ width: 10, height: 20 });
const circle = Shape.Circle({ radius: 5 });
```

The discriminant field is inferred from the union: it must be the unique common field whose variants carry distinct literal values.
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
Basically, structs are just values with a name, much like structs in other "systems languages": an alias to the struct's components.
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

Classes remain the TypeScript-shaped model for managed objects with identity, except of course without a prototype chain or any dynamic class shenanigans.
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

Class methods are concrete by default and must be `virtual` to enable virtual dispatch for an instance method in a subclass, and `abstract` to require an override before the class can be constructed.

```ds
abstract class Logger {
    abstract write(message: string): void;

    virtual flush(): void {}
}
```

Additionally, classes may be marked `final` to prevent downstream classes from extending the declaration.

```ds
final class PacketHeader {
    length: uint32;
}
```

### Arrays, Slices and Tuples

Destack supports richer sequence forms beyond the classic dynamic arrays - `T[]` / `Array<T>` with explicit slice, fixed array, and tuple forms.
Unfortunately, not much syntax was left here, so we had to adopt the slightly non-TS-y syntax forms of `[T]` and `[T; N]` for slices and fixed arrays, respectively.
(This is also why `.ds` does not support `.ts`-style array tuples `[A, B]` and tuples in `.ds` must always be explicit `(A, B)`)

| Forms | Representation | Meaning |
|------|----------------|---------|
| `T[]`, `Array<T>` | Collection class | Growable, homogeneous, dense sequence with capacity |
| `[T]`, `Slice<T>` | Slice header | Pointer plus length, no capacity |
| `[T; N]`, `FixedArray<T, N>` | Inline array | Exactly `N` elements stored in the value |
| `(A, B)` | Inline product | Heterogeneous sequence of owned values |

Dynamic arrays are managed objects with identity, while slices, fixed arrays, and tuples are semantically `struct`s (value/view forms).
Unlike Rust, Destack's `[T]` is sized and a first-class slice _value_, more akin to Go's slice header than Rust's unsized slice.
Also, unlike JavaScript, Destack does not permit holes in arrays or any other sequences, and indexing into `T[]` therefore returns `T`, not `T | undefined` (out-of-bounds indexing traps or errors depending on compiler options).

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

TypeScript already has `readonly`, but it is shallow, so in `.ds`, `readonly T` becomes a real _deep_ read-only view of `T`.
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
function size<comptime Wide: bool>(): Wide extends true ? 8 : 4 {
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

function collect<I: Iterator>(iter: I): I.Item[] { ... }
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
    ...
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
const result = if (let Some(value) = maybe) {
    value
} else {
    0
};

if (let (x, y) = point) {
    print(x + y);
}
```

### Closures

Closures generally work like TypeScript closures, capturing the surrounding lexical environment and preserving lexical `this`.
Destack supports an additional `@capture` annotation for controlling _how_ the environment is captured (both per closure and per binding):
 - `"borrow"`: captures borrowed access to the original binding (same as `&expr`)
 - `"copy"`: captures a copied value of the original binding (same as `expr.copy()` where `expr: Copy`)
 - `"move"`: transfer the binding into the closure, original becomes unavailable afterwards (same as `^expr`)

```ds
let count = 0;

@capture("borrow")
const next = () => {
    count += 1;
    return count;
};

let name = "Ada";

@capture("copy")
const greet = () => `hello ${name}`;

let socket = connect()?;

@capture("move")
const send = (message: string) => socket.write(message);
```

The short form sets the default for every captured binding.
For more complex cases, the object form of `capture` can override individual bindings, including `this`:

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
In standard managed land, this works as before, and managed values can be stored in parked frames just fine.
Because borrows must always point into valid memory and cannot change, borrows are valid only when the origin value is kept alive during suspension:

```ds
async function read(user: User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone(); // valid because `user` is kept alive during suspension
}
```

Frame-owned values can also be borrowed across suspension points:

```ds
async function read(user: ^User): Promise<string> {
    const name = &readonly user.name;
    await tick();
    return name.clone();
}
```

### Patterns

TypeScript has pattern based destructuring for arguments and assignment-like expressions, and Destack extends that idea into `match`, `if (let ...)`, `let ... else`, and `catch match` with a full suite of patterns for every type family:

| Family | Example | Meaning |
|--------|---------|---------|
| Wildcard | `_` | match and ignore the value |
| Binding | `value` | bind the matched value |
| Literal | `"ok"`, `0`, `true` | match one literal value |
| Tuple | `(x, y)` | destructure a tuple value |
| Array, slice, fixed array | `[head, ...tail]` | destructure indexed elements |
| Object | `{ kind: "ok", value }` | destructure a structural object |
| Struct | `Point { x, y }` | destructure a nominal struct |
| Newtype | `UserId(value)` | unwrap a nominal newtype |
| Enum | `State.Ready` | match a nominal enum variant |
| Union | `0 | 1 | 2` | accept any listed pattern |
| Rest | `...tail` | collect the remaining elements or fields |
| Default | `name = "guest"` | bind a fallback for missing destructured values |
| Must | `value!` | bind the non-nullish value |
| Ownership | `^value`, `&value` | bind an owned or borrowed view |
| Guard | `pattern if (condition)` | require an extra boolean condition |

`match` is the full form and checks exhaustiveness:

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
let Some(value) = maybe else {
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

For convenience and clarity, Destack supports `loop` as the explicit infinite loop form.
Like other loops, it can produce a value through `break <value>`.

```ds
const line = loop {
    const input = readInput();
    if (input == "quit") {
        break "done";
    }
    process(input);
};
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

As discussed in Types, structural interfaces keep normal TypeScript shape checking and mostly work exactly as expected: exact fields use offsets, methods use function targets, and adapted properties use generated accessors.
The internal "itab" table that maps this (commonly also called a "witness table") is generated automatically, and because it's a structural interface, writing `implements` on a structural interface is just an explicit check.

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

Union receivers are resolved per variant, and every variant must expose the member (otherwise it is an error).
If all variants resolve to the same implementation (i.e. function location) the call is static; otherwise the result type is the union of the selected return types.

```ds
struct TcpStream {
    write(chunk: [byte]): Result<usize, IOError> { ... }
}

struct MemoryBuffer {
    write(chunk: [byte]): Result<usize, never> { ... }
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
    MissingError { path } => Error(`missing config: ${path}`)
    FormatError { line } => Error(`bad format on line ${line}`)
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

Unlike in TypeScript, in Destack types can participate in custom tree tag behavior by implementing the `TreeTag` interface, and custom intrinsic types (lowercase tags like `<div>`) are programmable via `TreeTagBuilder`.
Essentially, `TreeTag` generalises `jsxFactory` and `TreeTagBuilder` generalises `jsxFragmentFactory`:
 - Uppercase or qualified tags resolve as value tags through normal value lookup and the `TreeTag` interface.
 - Lowercase unqualified tags resolve as intrinsic tags through the active `TreeTagBuilder`.

The active `TreeTagBuilder` comes from the compiler / target / profile options by default, but can be locally overridden with `module { ... }`.

```ds
import { HtmlTree } from "destack:ui/html";

module {
    tree: HtmlTree;
}
```


### Annotations and Decorators

Like TypeScript, Destack uses `@` for decorator-like constructs, but Destack distinguishes between "annotations" and "decorators", and also many more constructs can be targeted by decorators.
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

Destack modules can contain (up to) one static `module { ... }` directive block for source-level configuration that needs to be specific to a module.
Usually, we would configure this with the compiler / target / profile options, but sometimes it's helpful to override these options locally:

```ds
import { HtmlTree } from "destack:ui/html";

module {
    tree: HtmlTree;
    derive: [Debug, Clone];

    noHeap: true;
    noExceptions: true;
}
```

Every member value in the `module { ... }` block must be a static term, so imported providers and regular static term logic are permitted.
It should be noted that the configuration is - as the name implies - local to the specific module, and does not affect any other modules outside the current file.

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
function isPowerOfTwo(value: uint): bool {
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

    // ...
}
```

Of course, comptime results must also be lowerable into the target artifact.
Plain data such as numbers, strings, arrays, tuples, objects, structs, and enums are all fine, but dynamic runtime resources like pointers and handles and such don't work because we can't meaningfully serialize them.

#### Dynamic Code

Generating and evaluating arbitrary code is supported via `eval` and `new Function` at _compile-time_ by passing the static term of a string:

```ds
const source = comptime renderParser(grammar);
const parser = comptime eval(source);

const makeRoute = comptime new Function("request", "context", routeSource);
```

The source passed to `eval` or `new Function` must itself be available to comptime evaluation.
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
    ...
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

TypeScript, like most managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed on some local heap, and all value types are copied by default.
That is convenient, but sometimes we need to take direct control of memory, whether for better performance, or just to express invariants in the code.

Destack supports explicit, optional modifiers for controlling memory ownership and placement, inspired by Rust and Mojo with `^T` as the "owned" signifier.
Specifically, memory can be controlled along two axes:
- **Ownership** - who "owns" the value: managed (`T`), owned (`^T`), borrowed (`&T`, `&readonly T`, `&exclusive T`), or raw (`*T`).
- **Placement** - where the value is located: ambient by default, `shared` across Workers, explicit `"local"` in the type algebra, or some other target-defined space.

Plain `T` still behaves as the type's default representation, of course: value types are values, object types are managed references.
The two axes compose and commute freely, e.g. `shared ^T` and `^shared T` both mean an owned handle to a value in shared space, and `shared &T` is a borrow of a shared value.

### Ownership

Ownership determines who keeps a value alive, who is allowed to mutate it, and when and how it is eventually freed.
The usual explanation of "ownership" sounds more complex than it is, especially to developers uesd to "managed" languages, and _especially_ because Rust tradition conflates "exclusivity" and "mutability".
Unlike in Rust, in Destack we support _both_ multiple mutable borrows (`&T`) and exclusive mutable borrows (`&exclusive T`):

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

The "space" of a type and its corresponding memory region are usually just a logical distinction that is more about correctness and performance than physical representation.
For non-uniform memory targets, assigning specific memory spaces in one unified memory placement system is however quite convenient.

```ds
struct Request<T> {
    header: Header,
    body: T,
}

let localRequest: Request<Body>;          // ambient, default -> Request is worker-local heap
let sharedRequest: shared Request<Body>;  // explicit, shared -> Request is shared heap
```

Memory placement is contextual and all types are "ambient" by default, i.e., they come with no inherent placement.
Aggregate types are placed wherever their container is placed until some root either specifies placement explicitly (e.g., `WithPlace<T, ..>`, `shared T`) or we reach the top, which - as established - is `local` to the Worker's own local heap by default.
This "ambient placement" rule is also why we distinguish `Place` from `Space`: `Space` is concrete, while `Place` may also be `"ambient"`.

#### Shared Space

The shared space contains memory that is visible to all `Worker`s in the same `Runtime`.
Conceptually, `shared` is the typed, generalized version of the `SharedArrayBuffer` idea with the full type system and object graphs at our disposal:
- Local values may point to shared values.
- Shared values must not point directly into a local heap.

Shared placement, or any space placement, is **not** a synchronization primitive and does **not** imply atomic access, locking, actor isolation, `Sync`, or anything like it.
Libraries and strict profiles may require capabilities like `Send` and `Sync` for APIs that transfer or publish values for correctness, but `shared` itself is really only about placement.

### Static Space

Because Destack inherits the JS/TS Worker model for isolation, module-scoped constants are owned by each _Worker_ and are not actually process-global as they would be in most other languages.
For genuinely _shared_ process-global state, the binding _itself_ can be declared as `shared`.

| Form | Binding place | Value place | Meaning |
|------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

Note that making the binding itself as `shared` also types the value as `shared` (as it is illegal to point from shared storage into local storage anyway, this is convenient).

### Capabilities

Like other similar languages, Destack encodes synchronisation and memory primitives as (newtype) interfaces like `Copy`, `Clone`, `Send`, and `Sync`
(As said, these are separate from and orthogonal to ownership and placement.)

| Capability | Meaning |
|------------|---------|
| `Copy` | Value can be duplicated implicitly without changing ownership responsibilities. |
| `Clone` | Code can explicitly create another value, possibly by running code or allocating. |
| `Send` | Value can cross a Worker boundary. |
| `Sync` | References to shared values can be used concurrently through the type's own API. |

As discussed above, `shared T` means `T` lives in shared space; it does _not_ make `T` automatically `Sync`.
Userland APIs such as channels, Worker pools, atomics, locks, and actors can require `Send` or `Sync` when they need those stronger guarantees.

### Conversions

The rules for who can convert into what mostly follow from two facts:
- References must always be valid,
- Shared memory must not point into local memory.
(Raw pointers are explicit and unchecked.)

| From \ To | `T` | `&T` | `^T` | `*T` |
|-----------|-----|------|------|------|
| `T` | - | yes | no | explicit |
| `&T` | no | - | no | explicit |
| `^T` | no | yes | - | explicit |
| `*T` | no | reborrow | no | - |

The default type for a borrow is `&T`, and typing it as `*T` produces a raw pointer instead:

```ds
let user = new User();
let userBorrow: &User = &user;
let userPointer: *User = &user;
```

### Allocation

The primary typed construction path is `new`, which allocates heap storage, initializes a `T`, and produces the ownership form required by the destination type.

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

### Borrowing

There are different ways of ensuring memory safety, and Destack mostly follows the Rust tradition of using lifetimes to describe how borrowed `&T` values relate to their owners.
When borrowing a value with `&T`, the compiler needs to ensure that the borrow remains valid - that is, `T` must remain alive (must not be deallocated) while `&T` is active.

Like in Rust, even when working with borrowed values, most of the time all lifetimes are inferred correctly and we don't need to think too much.
Unlike in Rust, mutability is decoupled from borrowing: we can have multiple mutable borrows `&T` and readonly borrows `&readonly T` of the same `T` _at the same time_, as long as there is no concurrent `&exclusive T` borrow (which mirrors Rust's `&mut T`). 

| Form | Access |
|------|--------|
| `&readonly T` | may overlap, cannot mutate through the borrow |
| `&T` | may overlap, can mutate through the borrow |
| `&exclusive T` | cannot overlap another borrow of the same place, can mutate through the borrow |

The entirety of borrow checking behavior - ensuring that a borrow to some reference remains valid - follows from two simple rules:
1. a place cannot move or drop while an overlapping borrow is live;
2. a borrow cannot outlive the owner or access path it came from.

The rules are applied to access paths, so disjoint fields can be borrowed independently when the compiler can prove they do not overlap.
Borrow lifetimes are also based on use, not block scope: once the last use of a borrow has passed, the original place can be borrowed differently, moved, or dropped again.

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

Sometimes we need to spell out explicit lifetimes to clarify the relationship between owners and borrowsers, and for that purpose we have explicit `<L: Lifetime>` and `Borrowed<T, L>` generics. 
Instead of reifying lifetimes as special `'a`-style lifetime parameters, Destack's `<L: Lifetime>`s are standard static parameters that are also available to regular TypeScript-style type algebra:

```ds
function read(user: &User): string {
    // common borrowed parameters can use the surface `&T` form
    return user.name;
}

function first<T, L: Lifetime>(items: Borrowed<[T], L>): Borrowed<T, L> {
    // returned borrowed access is tied to the `items` lifetime
    return &items[0];
}

struct View<T, L: Lifetime> {
    // stored borrowed access makes the type carry the lifetime
    items: Borrowed<[T], L>;
}
```

The usual failure cases of borrowing rules usually have straightforward solutions, and sometimes simpler than in Rust since we can just bail out to managed ownership:

```ds
/* INVALID: borrow of a local owned Point cannot escape */
function escapedPoint(): &Point {
    let point = ^Point { x: 1, y: 2 };
    return &point;
}

/* VALID: owned Point is returned instead of borrowed access */
function ownedPoint(): ^Point {
    let point = ^Point { x: 1, y: 2 };
    return point;
}

/* VALID: borrow into a lifetime provided by the caller */
function borrowInput<L: Lifetime>(point: Borrowed<Point, L>): Borrowed<Point, L> {
    // return borrowed access tied to an input lifetime
    return point;
}
```

### Drop

Whenever the lifetime of a value ends and it is deallocated, Destack supports running a `Drop` finalizer (similar to Rust's `Drop`)
This happens both when the compiler inserted a drop for an owned local after its last use, when an owned field is being destroyed, and because the runtime is reclaiming an unreachable managed allocation.

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

Unlike Rust (and C++ RAII), Destack models  `Drop` and `Dispose` / `AsyncDispose` separately as two different axis (`Drop` is lifetime, `using` is lexical).
Indeed, because of this split `Drop` _can_ be performed eagler, improving efficiency, but also effectively precludes using `Drop` for lexical RAII style applications.  
In general, in Destack, `Drop` should be used to manage memory and memory-related cleanup, while `using` should be used for richer resource finalisation:

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

### Algebra

Type "algebra" is just a fancy way of saying that Destack supports querying and manipulating ownership and placement in its TypeScript-based type system, because _they_ are part of the type system.
Qualified surface forms like `readonly T`, `^T`, `&T`, `*T`, and `shared T` are sugar over a single normalized `Form`.
Plain `T` may remain unqualified, but algebra operators treat it as managed, mutable, and ambient when they need a default.
Unlike `Place`, `Access` is always concrete: plain `T` has access `"mutable"`, not some ambient access.

```ds
newtype Form<
    T,
    O: Ownership = "managed",
    P: Place = "ambient",
    L: Lifetime = never,
    A: Access = "mutable",
> = intrinsic;
```

All `Form`s are based on the common static evaluation machinery, and code can be generic over `Form<T, O, P, L, A>`, `WithSpace<T, S>`, or `PlaceIn<T, S>` without choosing a final address space, ownership, or access mode.

```ds
User            // unqualified, defaults to managed ambient
readonly User   // Form<User, "managed", "ambient", never, "readonly">
^User           // Form<User, "owned", "ambient">
^readonly User  // Form<User, "owned", "ambient", never, "readonly">
&readonly User  // Form<User, "borrowed", "ambient", L, "readonly">
&User           // Form<User, "borrowed", "ambient", L, "mutable">
&exclusive User // Form<User, "borrowed", "ambient", L, "exclusive">
*User           // Form<User, "raw", "ambient">
shared User     // Form<User, "managed", "shared">
shared ^User    // Form<User, "owned", "shared">
^shared User    // Form<User, "owned", "shared">
```

The helpers are the same few operations applied to each axis.
Constructors build a form from a base type:

```ds
Managed<User> satisfies Form<User, "managed", "ambient">;
Owned<User> satisfies Form<User, "owned", "ambient">;
Raw<User> satisfies Form<User, "raw", "ambient">;

shared User satisfies WithSpace<User, "shared">;
shared ^User satisfies WithSpace<^User, "shared">;
^shared User satisfies WithSpace<^User, "shared">;

Shared<^User> satisfies Form<User, "owned", "shared">;
Local<User> satisfies WithSpace<User, "local">;
Ambient<shared User> satisfies Form<User, "managed", "ambient">;
```

Borrowed forms additionally carry a lifetime:

```ds
type UserBorrow<L: Lifetime> = Borrowed<User, L>;
type UserReadonlyBorrow<L: Lifetime> = ReadonlyBorrowed<User, L>;
type UserExclusiveBorrow<L: Lifetime> = ExclusiveBorrowed<User, L>;
```

We provide builtin accessors to pull the axes back out of `Form`.
`AccessOf<T>` and `OwnershipOr<T, ..>` always return concrete values, while placement keeps the ambient distinction:
```ds
BaseOf<shared ^User> satisfies User;
OwnershipOf<^User> satisfies "owned";
OwnershipOr<User, "managed"> satisfies "managed";
AccessOf<User> satisfies "mutable";
AccessOf<readonly User> satisfies "readonly";
AccessOf<^readonly User> satisfies "readonly";
AccessOf<&exclusive User> satisfies "exclusive";
```

`Space` represents a concrete space like `"local"` or `"shared"` while `Place` means either a concrete `Space` or `"ambient"`, and ambient placement follows the containing context until a final layout is required:
```ds
PlaceOf<User> satisfies "ambient";
SpaceOf<^User> satisfies never;
PlaceIn<^User, "shared"> satisfies "shared";

PlaceOf<shared User> satisfies "shared";
SpaceOf<shared User> satisfies "shared";
PlaceIn<shared User, "local"> satisfies "shared";

PlaceOf<User | shared User> satisfies "ambient" | "shared";
SpaceOf<User | shared User> satisfies "shared";
PlaceIn<User | shared User, "local"> satisfies "local" | "shared";
PlaceIn<User | shared User, "shared"> satisfies "shared";
```

Predicates with `Is*` are convenience wrappers around those same accessors:
```ds
IsOwned<^User> satisfies true;
IsBorrowed<&User> satisfies true;
IsShared<shared User> satisfies true;
IsShared<^User> satisfies false;
IsSharedIn<^User, "shared"> satisfies true;
```

Rewriting one axis leaves the others alone:
```ds
WithSpace<^User, "shared"> satisfies Form<User, "owned", "shared">;
WithOwnership<shared User, "owned"> satisfies Form<User, "owned", "shared">;
WithPlace<shared User, "ambient"> satisfies Form<User, "managed", "ambient">;
WithAccess<User, "readonly"> satisfies readonly User;
WithAccess<^User, "readonly"> satisfies ^readonly User;
WithAccess<&User, "exclusive"> satisfies &exclusive User;
```

Except for the intrinsic `Form`, all the rest is just regular TypeScript-shaped type algebra.
That makes memory qualification just ordinary type-level computation: userland code can introspect and rewrite ownership and placement using the same type system for any other type.
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

Since ownership, access, lifetime, and placement are all part of `Form`, contracts and implementors get to be polymorphic and (somewhat) conditional over their ownership, space, and access, even on the receiver type.
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

### Synchronisation

The standard library provides the usual memory and synchronisation primitives on top of this unified memory system.
The full details are documented in the library, but the basics should be familiar to anyone with a systems-level background.

| Primitive | Contract |
|-----------|----------|
| `Box<T>` | unique heap ownership for `T`, with deterministic drop when `T: Drop` |
| `Rc<T>` | local shared ownership, non-atomic refcount, not transferable across Workers |
| `Arc<T>` | shared ownership, atomic refcount, transferable when `T` satisfies the required `Send` / `Sync` bounds |
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

### Modes

Modes generalise the idea of `module.tests.ds` into a more flexible `<module>.<mode>.ds` schema where `<mode>`s may include both well known contexts (`dev`, `prod`, `test`, `bench`, `lint`), and additional user-defined modes from `destack.json`. 
The base `module.ds` is always included when a mode is active, and additional `module.<mode>.ds` files are automatically included as if they were just at the end of the file.

For example, when importing `./user` with `test` mode active, both `user.ds` and `user.test.ds` are included:

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

In effect, this is just a file-level shortcut around static if gating `@if(import.meta.modes.includes("mode"))` for all declaratoins in a file.
Compiler, profile, and target options select active modes with `modes`:

```json:destack.json
{
    "modes": {
        "preview": { "extends": "dev" }
    },
    "compiler": {
        "modes": ["preview"]
    }
}
```

When a mode extends other modes, the inherited modes are included "before" the extending mode.

### Import Meta

`import.meta` exposes module and profile metadata during static and comptime evaluation.

| Field | Description | Type | Examples |
|-------|-------------|------|----------|
| `import.meta.url` | current module URL | `string` | `"file:///app/src/main.ds"`, `"https://example.com/mod.ds"` |
| `import.meta.path` | current local file path, when available | `string | undefined` | `"/app/src/main.ds"`, `undefined` |
| `import.meta.dir` | current local directory, when available | `string | undefined` | `"/app/src"`, `undefined` |
| `import.meta.output` | output artifact format | `Output` | `"js"`, `"wasm"`, `"native"` |
| `import.meta.platform` | target platform | `Platform` | `"linux"`, `"windows"`, `"web"` |
| `import.meta.target` | target family and ABI | `Target` | `{ family: "unix", arch: "x64", abi: "gnu" }` |
| `import.meta.runtime` | semantic runtime | `Runtime` | `"destack"`, `"js"` |
| `import.meta.modes` | active source graph modes | `readonly string[]` | `["test"]`, `["dev", "lint"]` |
| `import.meta.debug` | `debug` mode shorthand | `bool` | `true`, `false` |
| `import.meta.dev` | `dev` mode shorthand | `bool` | `true`, `false` |
| `import.meta.prod` | `prod` mode shorthand | `bool` | `true`, `false` |
| `import.meta.test` | `test` mode shorthand | `bool` | `true`, `false` |
| `import.meta.bench` | `bench` mode shorthand | `bool` | `true`, `false` |
| `import.meta.lint` | `lint` mode shorthand | `bool` | `true`, `false` |
| `import.meta.env` | configured build environment | `{ readonly [key: string]: string | bool | number }` | `{ NODE_ENV: "production", FEATURE_X: true }` |

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
