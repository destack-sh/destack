# "Language"

The Destack language (`.ds`) and toolchain, colloquially "TypeScript++", are a superset of "strict modern" TypeScript with support for `.ts` and `.tsx` files, native AOT compilation and a fully integrated language toolchain, _and_ it can also compile nicely to standard JS/TS targets.
We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated stack (Destack), and thus by "language" ("TypeScript++") we mean much more than "just" a syntax form: a language, a runtime, a toolchain, plugins, and ultimately, a way of programming.

## "TypeScript++"

We're very early in software, and we're still figuring out how to build optimal, correct, and integrated software systems.
Over 50 years, we have grown more and more layers of software sediment and need ever _more_ tools to manage the get any code out the door, and yet confidence and performance have plummeted.

We believe the best possible stack is the most integrated one, and it must truly span the entire lifecycle: the language itself, the toolchain with linters and formatters, a VM, compiler, runtime, and basically anything that touches the code.
Only TypeScript is seriously close to being a universal software foundation, because it runs directly on the web, and the web is the most ubiquitous application platform.
The TypeScript ecosystem has good - if not perfect - answers to all modern software needs, from great developer tools to rich interactive frontends to quite _decent_ and performant backends.

If you remove all the JS baggage and dynamic prototype mess, modern TypeScript is surprisingly close to a fully AOT-compilable language (and most browsers retrofit compilation internally already based on this assumptions).
Embracing TypeScript and "the web ecosystems" lets us build a new toolchain that truly covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be completely free of JS overhead and (some) historic baggage.

## Compatibility

**Destack is a superset of "modern strict" TypeScript**.
The intended use case is for TS-shaped code that can be type checked, optimized, and compiled ahead of time without pretending every value might turn into a different shape at runtime; in other words, Destack is based on the sound subset of TypeScript.
More specifically, Destack excludes legacy syntax and all sorts of dynamic shapes and protocols that are not statically fixed / knowable.

### Syntax

Some JS/TS syntax and legacy behavior is either ambiguous, obsolete, or just not worth carrying forward.

- **Ambiguous generic arrow**: `<T>() => ...` is ambiguous in `.tsx`, and `.ds` inherits this since it supports TSX syntax natively.
- **Sequence expressions**: `(A, B, C)` is - confusingly - a "sequence expression" in JS, which nobody ever really types out by hand, and `.ds` instead uses `(A, B, C)` for explicit tuples.
- **Enum coercion**: `enum Level { A = 1, B = 2, C = 3 }` is _just_ an alias in TypeScript, but we do _not_ coerce `Level.A` to `number` without an explicit cast.
- **Flow and JSDoc _typing_**: We support TypeScript only.
  Where Flow and TS overlap we obviously support both, but we do no special JSDoc analysis.
- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript.
  Non-strict ("sloppy mode") behaviors like duplicate function declarations, `arguments` magic, `caller` / `callee`, or `yield` as an identifier are not supported.
- **`any`**: `.ds` uses `unknown` as the top type.
  TypeScript `any` is rejected because it makes arbitrary property access, calls, and assignments appear valid without proof.
  Existing TS code must narrow through `unknown` or use explicit casts at interop boundaries.
- **Definite assignment assertions**: `let x!: T` and `field!: T` are rejected in `.ds`.
  Locals and fields must be actually initialized before use, either by an initializer or by ordinary definite assignment analysis.
- **XML namespace resolution**: Destack does not implement XML `xmlns` namespace binding semantics.
  Namespaced tree tags like `<svg:path />` are treated as intrinsic string tag names (`"svg:path"`).

### Shapes

Dynamic shapes and strict native compilation do not like to mix.
In `.ds`, values have statically known shape, and classes have a fixed static object model instead of some mutable JS constructor object.

- **Declaration expressions**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with proper AOT compilation.
- **Dynamic code generation**: Dynamic _runtime_ `eval` / `new Function` / class generation are in conflict with a strict AOT model and unsupported, **but** Destack supports explicit `comptime eval` / `new Function`.
- **Prototype objects**: `.prototype`, `.__proto__`, `.constructor`, `Object.getPrototypeOf`, `Object.setPrototypeOf`, and `Object.create(proto)` all rely on the prototype-based object model and are not supported.
- **Shape mutation**: `delete`, `Object.defineProperty`, `Object.defineProperties`, `Reflect.defineProperty`, `Reflect.deleteProperty`, and shape-changing `Object.assign` are forbidden.
- **Metaobject dispatch**: `Proxy` and most `Reflect.*` APIs exist to intercept or emulate dynamic object behavior, so they are also unsupported.
- **CommonJS mutation**: `require`, `module.exports`, and require-cache monkeypatching are dynamic module-shape features, and are also (mostly) unsupported.
- **Circular inference**: Destack does not support circular inference _across_ modules. Modules may export types they can establish from local declarations _and_ imports, and downstream modules may build on those exports, but downstream uses do not refine upstream declarations.

### Protocols

JS also has a lot of behavior where the runtime secretly calls user code through special names or symbols.
Destack instead uses typed protocols, declared members, static members, and extensions instead.

- **Thenables**: `await` does not mean "anything with a `.then` property". It targets `Promise<T>` or another typed async protocol.
- **Coercion hooks**: `valueOf`, `toString`, and `Symbol.toPrimitive` do not participate in implicit object coercion. Use explicit conversions, formatting/display protocols, interpolation, or operator overloads.
- **Loose equality coercion**: Object coercion through `==` and `!=` is not part of `.ds`.
- **Well-known symbol magic**: `Symbol.hasInstance`, `Symbol.species`, `Symbol.isConcatSpreadable`, and similar hooks are not language semantics. Iteration can still exist as a typed `Iterable<T>` protocol, even if a JS target lowers it to symbols.
- **Implicit call and construct hooks**: Arbitrary `[[Call]]`, `[[Construct]]`, and `Function.prototype.call` / `apply` / `bind` are not implicit members. Callable and constructable values must have declared callable or constructable types.

# Language

"TypeScript++" is a superset of "strict modern" TypeScript, which essentially means that existing TypeScript (and TSX!) _just works_ **if** it follows our strict TypeScript-based type system.
Fortunately, strict TypeScript is already a best practice, and it's what you get when enabling the recommended soundness flags in TSC (mostly).
TypeScript++ adds some new features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do for frontend-shaped software, but for the entire software stack including "systems software".

There are solid arguments that a language should be minimal (like Zig or Go or even C), but we do not believe "language minimalism" to be pragmatic for the universal language and toolchain we want.
That said, TypeScript is already not a simple language, and any additional language features risk becoming unwieldy.
We embrace this tradeoff, and as needed _some_ additions for serious systems programming, we took the opportunity to round out the language with modern ergonomics like patterns, operator overloading, reflection, and comptime.

## Types

Destack extends TypeScript's type system with precise primitives, nominal types ("`newtype`s"), value types ("`struct`s"), tuples, ergonomic constraints, and some additional niceties.

### Primitives

TypeScript inherits its primitive types from JavaScript: `object`, `string`, `boolean`, `number`, `bigint`, and `symbol`, plus the `null` and `undefined` sentinels.
Destack extends and refines the primitive type system:

- precise numeric types beyond `number`, with variable-width signed and unsigned integers (`int8`, `uint32`, `int17`) as well as single and double precision floats (`float32`, `float64`)
- pointer-sized integers, i.e. integers as wide as the target pointer size, spelled `isize` and `usize`
- `number` as an alias to `float64`
- `character` as a single Unicode scalar value, distinct from `string`
- `unknown` as the explicit top type
- `never` as the explicit bottom type
- no `any`

Following the spirit of TypeScript's widening rules, numeric literals start as exact values and can flow into any numeric type that can represent them.
When no specific numeric context fits, the literals widen as usual to plain `number` (i.e. `float64`).

```ds
const id: uint64 = 12345;
7 satisfies uint3;
7 satisfies uint2; // error

const balance: float32 = 100.50;
const n: number = 1.0;
n satisfies float64;

const initial: character = 'A';
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

Extensions can be added to any **nominal type**, so all types like `struct`, `class`, `enum`, `newtype`, whether defined locally or in a foreign / imported module.
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

Enums are nominal aliases to a set of constants, just like in Typescript, except that Destack's enums do not implicitly cast to their backing type and explicit conversions are required for the backing value type.
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
Using Destack's nominality `newtype` and the builtin `Tagged` derive provider, plain old discriminated unions become pretty presentable sum types:

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "rectangle"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

extension of Shape {
    static DEFAULT = Shape.Rectangle({ width: 10, height: 20 });
    
    variant() {
        match (this) {
            Shape.Rectangle(_) => "rectangle"
            Shape.Circle(_) => "circle"
        }
    }
}

// create values of tagged newtype unions with <Type>.<Variant>
const rectangle = Shape.Rectangle({ width: 10, height: 20 });
const circle = Shape.Circle({ radius: 5 });
```

The discriminant field is inferred from the union: it must be the unique common field whose variants carry distinct literal values.
By default, string discriminants are exposed as `UpperCamelCase` constructor names, the other supported naming policies are:

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

For composition, structs support embedding other structs directly in line:

```ds
struct Transform {
    position: Vec3;
    rotation: Quat;
}

struct Player {
    // embeds Transform's fields
    ...Transform;
    health: int;
}
```

### Classes

Classes remain the TypeScript-shaped model for managed objects with identity, except of course (like all objects) without an prototype chain or dynamic class shenanigans.

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

Class fields use strict initialization: every required instance field must be initialized by its declaration, a parameter property, or every constructor path.
(Optional fields do not need eager initialization.)

### Arrays, Slices and Tuples

Destack supports richer sequence forms beyond the classic dynamic arrays - `T[]` / `Array<T>` with explicit slice, fixed array, and tuple forms.
Unfortunately, not much syntax was left here, so we had to adopt the slightly non-TS-y syntax forms of `[T]` and `[T; N]`.
(This is also why `.ds` does not support `.ts`-style array tuples `[A, B]`; tuples must be explicit `(A, B)`)

| Forms | Meaning |
|------|---------|
| `T[]`, `Array<T>` | Dynamic, homogeneous, dense array |
| `[T]`, `Slice<T>` | Runtime-length homogeneous view into dense storage |
| `[T; N]`, `FixedArray<T, N>` | Fixed, owned sequence of values |
| `(A, B)` | Sequence of heterogenous, owned values |

Unlike JavaScript, Destack does not permit holes in arrays (or any other sequences)
Indexing into `T[]` therefore returns `T`, not `T | undefined`; out-of-bounds indexing traps or errors depending on compiler options.

```ds
let xs: int32[] = [1, 2, 3];
let ys: Array<int32> = [1, 2, 3];
```

Fixed arrays are homogeneous arrays whose length is statically known and part of the type.
They are inline value/layout types by default, and definitionally cannot grow.

```ds
type Block = [uint8; 4096]; // 4KB of uint8
type Vec3 = [float32; 3];   // 3 float32s

let rgb: [uint8; 3] = [255, 128, 0];
let zeroes: [uint8; 32] = [0; 32];
```

Fixed arrays and slices also work directly in patterns.
Fixed array patterns know their length statically, while slice patterns can use a rest binding for the tail:

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

As in TypeScript with `readonly` (or Rust with `mut` in inverse), `readonly` does not directly affect runtime behavior or freeze anything, it's just a semantic convention.

### Generics

Destack keeps TypeScript-shaped generics: inference, constraints, defaults, conditional types, mapped types, indexed access types, and the rest of the usual machinery.
The main addition is that generic parameters can also be _values_ that are then substituted into expressions and are available during inference.
To distinguish static value parameter from static type parameters, we use the `comptime` modifier:

```ds
type Buffer<comptime N: uint> = [uint8; N];
```

Generic bounds can use the cleaner `<T: Constraint>` form, just like dynamic parameters.
Unlike with `comptime <expr>` (discussed later), the `comptime` modifier merely means that `N` is a generic value parameter that has to be evaluatable during inference.

```ds
function copy<T, comptime N: uint>(src: [T; N]): [T; N] {
    let dst: [T; N];

    for (let i = 0; i < N; i++) {
        dst[i] = src[i];
    }
    dst
}
```

Type inference is local and flows outward - you can "import" inference from other modules, but this only goes one way: each module can infer static types and values from its own declarations and imports, and downstream modules can use what it exports.
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

One fundamental task of the language is turning text into something executable, and along the way we have to decide what every symbol actually means.
More specifically, since unlike TypeScript, Destack actually _compiles_, we need to figure out during "compile time" the final type of each value and fill in values for all the known constants.
To do this, the "evaluation time" of the program is conceptually split into three worlds (that only flow forward):

| World | Meaning | Example |
|-------|---------|---------|
| Static | types, values, and relations known while checking | `T`, `N`, `this.Width`, `T extends string` |
| Comptime | ordinary code explicitly evaluated by the compiler | `comptime factorial(10)` |
| Runtime | ordinary program execution | `readFile(path)`, `worker.postMessage(msg)` |

The statically known forms known to inference are called **static terms**: static evaluation is what we do automatically during inference, and it is restricted to a small subset of the language (like TypeScript type operators), and it can _not_ execute `comptime <expr>` expressions.
Static terms can include primitive inputs, imported facts, and expressions built from other static terms:

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
| Module and profile metadata | `import.meta.target.os` |
| Type operators and relations | `keyof T`, `T[K]`, `T extends string`, `T implements I` |
| Type/layout intrinsics | `sizeOf<T>()`, `alignOf<T>()` |

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

    @if(Wide == false)
    return 4;
}
```

### Associated Types and Constants

Associated types and constants contribute static members to a type that can be reused within the type and implementors but does not need to be exposed to every single caller.
Both associated types and constants also work in abstract types, and they do not occupy any instance space on the type.
As described above, all statically known types and constants use the same static evaluation logic, and thus associated types and constants also mix with generic parameters, conditional types, decorators, and so on.

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

Associated types can have their _own_ generic parameters with the same generic parameter forms as ordinary declarations, including type parameters and `comptime` value parameters (these are generic associated types, often called GATs.)

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
Like `static` members, `comptime const`s require no instance storage, but unlike `static` members, `comptime const` are statically evaluated during compilation.

```ds
interface RegisterBlock {
    comptime const Width: uint;

    read(): [uint8; this.Width];
    write(bytes: &[uint8; this.Width]): void;
}
```

As described above, associated members participate in the same static evaluation world, and so associated members can express dependent types and values.

```ds
interface Matrix<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Bytes = [uint8; this.Width];
}

function read<M: Matrix<unknown>>(bytes: M.Bytes): [uint8; M.Width] { ... }
```

### Constraints

`where` clauses for readable generic constraints in complex types:

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

In TypeScript, control flow forms like `if` are statements, and you need a ternary or temporary to get a value out.
That's fine but makes match-like patterns more complex to express, so Destack lets block forms like `if` and `match` produce values.
The last expression (no trailing `;`) becomes the value of the overall expression.

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

Building on statements-as-expressions, if-let expressions enable nice sugar for matching a value with a refutable pattern in a conditional.
Bindings from the pattern are then available in the positive branch, and this composes with TS flow typing as you would expect.

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

`switch` remains the TypeScript statement form with fallthrough, `case`, `default`, and ordinary `break`.
Use `match` when you want patterns, exhaustiveness, guards, or a value.

### Closures

Closures generally work like TypeScript closures: capture the surrounding lexical environment and preserve lexical `this`.
Destack supports an additional `@capture` annotation for controlling how the environment is captured (both per closure and per binding):
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
Object form can override individual bindings, including `this`:

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

Async functions and generators are closures that can pause and be resumed at a later point via `Continuation`s.
When paused, the runtime parks the live frame in a Worker-local continuation handle.
`Promise`, `Generator`, and `AsyncGenerator` are (well known) standard library types around the simpler `Continuation` primitive:

| Form | Meaning |
|------|---------|
| `ContinuationHandle` | Worker-local handle to a parked frame |
| `Promise<T>` | Worker-local async result object |
| `Generator<Y, R, N>` | Worker-local suspended generator |
| `AsyncGenerator<Y, R, N>` | Worker-local suspended async generator |
| produced `T` | value eventually produced by async code |

As in TypeScript, `await` and `yield` are the suspension points for the `Promise`s and `Generator` (and `AsyncGenerator`) coroutines.
When using standard managed values, these coroutines work exactly as before with no special regard for memory ownership.
When using owned and borrowed values, beware that borrows cannot live safely across suspension points:

```ds
async function read(user: User): Promise<string> {
    const id = user.id;
    await tick();

    const name = &user.name; // borrow after suspension
    return name.clone();
}
```

### Patterns

TypeScript already has pattern based destructuring for arguments and assignment-like expressions, so richer patterns fit in quite naturally.
Destack extends that idea into `match`, `if (let ...)`, `let ... else`, and `catch match` with a full suite of patterns for every type family:

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

The type of a match expression is the joined type of its arm bodies.

```ds
declare point: Point;
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at ${x}, ${y}` // irrefutable if point: Point
}
```

Some patterns are irrefutable, which means they always match, and then we do not need any alternative branches, like with `_` or destructuring of known shapes.
Refutable patterns require some fallback such that all branches are covered: a `match` fallback arm, an `else` branch for `if (let ...)`, or an `else` continuation for `let ... else`.

```ds
declare point: Point;
match (point) {
    Point { x: 0, y } => "vertical"
    Point { x, y: 0 } => "horizontal"
    _ => "neither" // required fallback
}

declare maybe: Option<int32>;
let Some(value) = maybe else {
    return Result.err("missing value");
};
```

### Guards

Guards are boolean expressions that can refine types in the branch where they are known.
That includes the familiar TypeScript forms whose meaning the compiler can check directly: `typeof value == "string"`, `"name" in value`, and `instanceof`.
Destack also adds an additional `value is T`, which asks whether the current runtime representation of `value` carries the case or identity for `T`:

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

The `using` (and `await using`) feature - officially known as explicit resource management - is a [stage 3 TC39 proposal](https://github.com/tc39/proposal-explicit-resource-management).
We just follow that proposal with `using` / `await using` as explicit scoped cleanup, but of course using nominal interfaces instead of `Symbol`s:
- `using` accepts `Dispose | null | undefined`.
- `await using` accepts `AsyncDispose | Dispose | null | undefined`, and falls back to synchronous disposal when the resource only implements `Dispose`.
- `null` and `undefined` are ignored, following the spec.

Resources are cleaned up at lexical scope exit in LIFO order, and `await using` runs async cleanup when required.
Cleanup - that is, the dispose function - runs when the scope exits for any reason: fallthrough, `return`, `break`, `continue`, `throw`, or `?`.
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

When overflow / wrapping policy is part of the algorithm, the expression should say so directly, so Destack provides Zig-style wrapping and saturating arithmetic for integer code:
 - `+%`, `-%`, and `*%` wrap modulo the integer's range.
 - `+|`, `-|`, and `*|` clamp to the integer's minimum or maximum value.
(These forms are not overloadable.)

Dereference operators are a little different from the main "value-shaped" operators.
`ReadonlyDereference` and `Dereference` project one access form into another access form, preserving ownership, placement, mutability, and borrow regions.

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

"Dispatch" is how calls, member accesses, and overloadable operators select an implementation to invoke.
The selection rule is TS-derived: build the candidate set, keep candidates compatible with the arguments as written, then pick the first one in declaration order:

#### Overloads

Overload resolution follows source order.
Unlike in TypeScript, there may be multiple overloaded _implementations_ for the same name, but the selection rule is the same as in TypeScript.

```ds
function parse(input: string): int32 {
    return parseInt(input);
}

function parse(input: int32): int32 {
    // legal, actually different implementation!
    return input;
}
```

Members work the same way (after receiver lookup): inherent members first, then visible extension members in declaration order.

#### Operators

For overloadable operators, the operator decides the interface to check, and the left operand is the receiver (matching how it is written in the interface implementation).
Binary operators do not fall back to the right operand.
If both operand orders should work, both receiver implementations must exist.

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

Exceptions are deeply enmeshed into TypeScript, and therefore Destack supports them, too (alas).
However, Destack also supports and strongly encourages **Result-first error handling** inspired by Rust: recoverable errors use `Result<T, E>`, integrate with `try` / `catch`, and can be opened with `?`, `??`, and postfix `!`.

#### Result

Destack provides `Result<T, E>` as the primary error handling mechanism.
The `Result` type is defined as regular code:

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

It should be noted that nullish values on the failure side remain in the failure side.
The various operators act differently on the failure case:

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

The try-coalesce operator `??` handles the same shape locally with a fallback instead of letting it leave the expression.
The result then is the non-nullish opened success type joined with the fallback type.

```ds
declare const defaultConfig: Config;

declare function loadConfig(): Result<Config, IOError> | null;
const a = loadConfig() ?? defaultConfig;
a satisfies Config;

declare function loadMaybeConfig(): Result<Config | null | undefined, IOError | null> | undefined;
const b = loadMaybeConfig() ?? defaultConfig;
b satisfies Config;
```

Nested `Try` values inside the success type also stay wrapped:

```ds
declare function loadNested(): Result<Result<Config, IOError>, IOError>;

const c = loadNested() ?? defaultConfig;
c satisfies Result<Config, IOError> | Config;
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

Propagation is the matching failure-side operation.
When a failure leaves the current function, the enclosing return type must implement `FromFailure<F>` for the propagated failure type.

```ds
newtype interface FromFailure<F> {
    static fromFailure(failure: F): this;
}
```

#### Try, Catch and Finally

The `try`/`catch` syntax handles both exceptions and explicit `Try` propagation:

```ds
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e) {
    log("failed to read config:", e)
}
```

The example uses `Result`, but any type implementing `Try` behaves the same:
- `try` does not implicitly unwrap `Result` values
- Use `?` inside the block to propagate `Try` failures into the catch
- Use `??` inside the block when the failure should be handled locally with a fallback
- When a `?` is inside a `try` with a catch, `FromFailure` is not required

When the propagated failures are statically known, `catch match` can branch on them directly:

```ds
try {
    readConfig()?; // -> Result<void, MissingError>
    parseConfig()?; // -> Result<void, FormatError>
} catch match (failure) {
    MissingError { path } => Error(`missing config: ${path}`)
    FormatError { line } => Error(`bad format on line ${line}`)
}
```

### Trees (TSX)

`.tsx` has proven to be a great way of building UIs and has even seen adoption for other tree-shaped data structures.
Destack (`.ds`) files natively support `.tsx` like constructs:

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
Essentially, the familiar split remains, with `TreeTag` generalising `jsxFactory` and `TreeTagBuilder` generalising `jsxFragmentFactory`:
 - Uppercase or qualified tags resolve as value tags through normal value lookup and the `TreeTag` interface.
 - Lowercase unqualified tags resolve as intrinsic tags through the active `TreeTagBuilder`.

The active `TreeTagBuilder` comes from the compiler / target / profile options by default, but can be locally overriden via the `module { ... }` directive block.

```ds
import { HtmlTree } from "destack:ui/html";

module {
    tree: HtmlTree;
}
```

### Annotations and Decorators

Like TypeScript, Destack uses `@` for decorator-like constructs, but Destack supports both "annotations" and "decorators", and many more things can be decorated.
The syntax is unified, the form - the thing pointed to in `@<expr>` - decides:
 - **Annotations** are _values_ like `newtype`s. They add typed metadata to the target, but don't directly change the target's behavior.
 - **Decorators** are _logic_ following some protocol that contribute code or change the analyzed shape in some bounded way.

Annotations are "inert" by default, that is, they don't do anything until either some userland construct or the toolchain give them special meaning (like with `@capture`).

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

#### Patch

Decorators are just nominal values, like annotations, but they implement the `Patcher` protocol for the target they are applied to.
This also means that decorator configuration is just regular values:

```ds
newtype memoize = {
    capacity?: uint;
};

@memoize({ capacity: 1024 })
function load(id: UserId): Result<User, Error> {
    ...
}
```

The `Patcher` algebra uses small bounded `Patch`es: add, replace, rename, remove.

```ds
extension<F> of memoize implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext<F>, options: this): Patch[] {
        const replacement = comptime eval<Declaration>(ds`
            function ${context.name}(...args) {
                ...
            }
        `);

        return [
            PatchReplace({
                symbol: context.target,
                declaration: replacement,
            }),
        ];
    }
}
```

Patch expansion is not recursive: declarations that implement `Patcher` are analyzed before patching, and cannot themselves be changed by patchers or derive.

#### Derive

Similar to Rust, Destack supports `@derive` providers for extending certain declarations at compile time.
Unlike in Rust, a derive provider is just a nominal decorator that happens to implement the `Patcher<Target>` interface, and `derive`-like "macros" do not need to be implemented in a different package (or "crate") or in any special syntax.

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

At the library level, a derive provider is just a nominal provider value returning patches:

```ds
newtype interface Patcher<Target> {
    static patch(target: Target, context: PatchContext<Target>, config: this): Patch[];
}

newtype Tagged = () | {
    case?: TaggedCase;
    names?: TaggedNames;
};

extension<Target> of Tagged implements Patcher<Target> {
    @intrinsic
    static patch(target: Target, context: PatchContext<Target>, config: this): Patch[];
}
```

#### Static If

Destack also supports a special `@if` decorator that gates the inclusion of certain nodes based on a static term.
When the condition is false, the annotated item is (in effect) removed from the instantiated shape.

```ds
interface FileSystem {
    open(path: string): Result<File, IOError>;

    @if(import.meta.target.os != "windows")
    chmod(path: string, mode: uint16): Result<void, IOError>;

    @if(import.meta.target.os == "windows")
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
    tree: HtmlTree; // configure the current tree tag builder
    derive: [Debug, Clone]; // configure the default auto derives
}
```

The fields must be static terms, may reference imports, and are resolved before ordinary analysis of the module body.

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

It is important to note that functions do not declare themselves as either "comptime" or "runtime": the same function can run at compile time when all inputs are static, and at runtime when some input is only known at runtime:

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
```

Of course, comptime results must also be lowerable into the target artifact.
Plain data such as numbers, strings, arrays, tuples, objects, structs, and enums are all fine, but dynamic runtime resources like pointers and handles and such are not allowed.

#### Dynamic Code

Generating and evaluating arbitrary code is supported via `eval` and `new Function` at _compile-time_, just by passing the static term of a string to a comptime-scoped `eval` or `new Function` call:

```ds
const source = comptime renderParser(grammar);
const parser = comptime eval(source);

const makeRoute = comptime new Function("request", "context", routeSource);
```

The source passed to `eval` or `new Function` must itself be available to comptime evaluation.
Generated code is parsed and typechecked as `.ds`, attached to the same module graph as a virtual source file, and tracked for diagnostics and artifact caching.

## Memory

TypeScript, like most managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed on some local heap, and all value types are copied by default.
That is convenient, but sometimes we need to take direct control of memory, whether for better performance, or just to express invariants in the code.

Destack supports explicit, optional modifiers for controlling memory ownership and placement, inspired by Rust and Mojo with `^T` as the "owned" signifier.
Specifically, memory can be controlled along two axes:
- **Ownership**: who "owns" the value: managed (`T`), owned (`^T`), borrowed (`&T`), or raw (`*T`).
- **Placement**: where the value is located: ambient by default, `shared` across Workers, explicit `"local"` in the type algebra, or some other target-defined space.

Plain `T` still behaves as the type's default representation, of course: value types are values, object types are managed references.
The two axes compose and commute freely, e.g. `shared ^T` and `^shared T` both mean an owned handle to a value in shared space, and `shared &T` is a borrow of a shared value.

### Ownership

Ownership decides who keeps a value alive, who is allowed to mutate it, and when and how it is eventually freed.
Each ownership form has a corresponding normalized representation in our little "type algebra" (see [Algebra](#algebra)).

| Form | Ownership | Liveness | Meaning |
|------|-----------|----------|---------|
| `T` (value base) | value | owned by its containing storage | inline value |
| `T` (object base) | managed | keeps the referent alive | managed heap handle (GC) |
| `^T` | owned | owns the referent | unique owned handle |
| `&T` | borrowed | requires liveness | semantic borrow or projection |
| `*T` | raw | does not keep anything alive | unsafe typed pointer |

```ds
let a: User = new User();  // managed handle
let b: ^User = new User(); // owned handle
let c: &User = &a;         // borrowed handle
let d: *User = &a;         // raw handle (unchecked)
```

### Space

Space defines where some value is actually located in memory, and since Destack follows web and JS/TS convention, we use the `Worker`-local heap as the default main memory space.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space, and user and library code can almost always just pretend spaces don't exist.

The "space" of a type and its corresponding memory region are often just a purely logical separation that is much more about correctness (and somewhat about performance) than about physical representation.
For non-uniform memory targets, assigning specific memory spaces in one unified memory placement system is however quite convenient.

```ds
struct Request<T> {
    header: Header,
    body: T,
}

let localRequest: Request<Body>;          // ambient, default -> Request is worker-local heap
let sharedRequest: shared Request<Body>;  // explicit, shared -> Request is shared heap
```

Memory placement is contextual: all types are "ambient" by default, i.e., they come with no inherent placement.
Aggregates are placed wherever their container is placed until someone either specifies placement explicitly (e.g., `WithPlace<T, ..>`, `shared T`) or we reach the root, which is `local` to the Worker's own local heap by default.
This is why we distinguish `Place` from `Space`: `Space` is concrete, while `Place` may also be `"ambient"`.

#### Shared Space

The "shared space" is shared memory visible to all `Worker`s in the same `Runtime`.
Conceptually, `shared` is the typed, generalized version of the `SharedArrayBuffer` idea with the full type system and object graphs at our disposal:
- Local values may point to shared values.
- Shared values must not point directly into a local heap.

It should be noted that shared placement - or any space placement - is **not** a synchronization primitive in itself, and does **not** imply atomic access, locking, actor isolation, `Sync`, or anything like it _by itself_.
It's just a name for a region of memory, nothing more.
Libraries and strict profiles may require capabilities like `Send` and `Sync` for APIs that transfer or publish values, but `shared` itself is only placement.

### Capabilities

Like many other languages, Destack encodes synchronisation and memory primitives as (newtype) interfaces like `Copy`, `Clone`, `Send`, and `Sync`
(These are separate from and orthogonal to ownership and placement.)

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
(Raw pointers are your own dirty business.)

| From \ To | `T` | `&T` | `^T` | `*T` |
|-----------|-----|------|------|------|
| `T` | - | yes | no | unsafe |
| `&T` | no | - | no | unsafe |
| `^T` | no | yes | - | unsafe |
| `*T` | no | reborrow | no | - |

### Statics

Because Destack inherits the JS/TS Worker model for isolation, module-scoped constants are owned by each _Worker_ and are not actually process-global as they would be in many other languages.
For genuinely shared process-global state, the binding _itself_ can be declared as `shared`.

| Form | Binding cell | Value place | Meaning |
|------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

### Allocation

The primary typed construction path is `new`, which allocates heap storage, initializes a `T`, and produces the ownership form required by the destination type.

```ds
let a: User = new User();   // managed
let b: ^User = new User();  // owned
```

### Borrows, Regions and Suspension

A region is one compiler-known lifetime relation for one borrow.
Explicit region spelling is only needed when a signature must relate returned borrows to input borrows.

```ds
&T                 // surface syntax
Borrowed<T, _>     // normalized form

@lifetime("a") &T
Borrowed<T, "a">
```

### Algebra

Type "algebra" here is just a fancy way of saying Destack supports querying and manipulating ownership and placement in its type system, because _they_ are part of the type system. 
Qualified surface forms like `^T`, `&T`, `*T`, and `shared T` are sugar over a single normalized `Form`.
Plain `T` may remain unqualified, but algebra operators treat it as managed ambient when they need a default.

```ds
newtype Form<
    T,
    O: Ownership = "managed",
    P: Place = "ambient",
    R = never,
> = unknown;
```

All these `Form`s are based on the common static evaluation machinery, and code can be generic over `Form<T, O, P, R>`, `WithSpace<T, S>`, or `PlaceIn<T, S>` without choosing a final address space or ownership.

```ds
User            // unqualified, defaults to managed ambient
^User           // Form<User, "owned", "ambient">
&User           // Form<User, "borrowed", "ambient", R>
*User           // Form<User, "raw", "ambient">
shared User     // Form<User, "managed", "shared">
shared ^User    // Form<User, "owned", "shared">
^shared User    // Form<User, "owned", "shared">
```

The provided convenience memory algebra operators are just the those same families applied to those axes.
Constructors build a form from a base type:

```ds
Managed<User> satisfies Form<User, "managed", "ambient">;
Owned<User> satisfies Form<User, "owned", "ambient">;
Borrowed<User, "a"> satisfies Form<User, "borrowed", "ambient", "a">;
Raw<User> satisfies Form<User, "raw", "ambient">;

shared User satisfies WithSpace<User, "shared">;
Shared<^User> satisfies Form<User, "owned", "shared">;
```

Accessors pull the axes back out.
The `*Of` family returns `never` when that axis is not explicit on the input, while `*Or` applies a default:
```ds
BaseOf<shared ^User> satisfies User;
OwnershipOf<^User> satisfies "owned";
OwnershipOr<User, "managed"> satisfies "managed";
RegionOf<Borrowed<User, "a">> satisfies "a";
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
AsBorrowed<shared User, "a"> satisfies Form<User, "borrowed", "shared", "a">;
```

Except for the intrinsic `Form`, all the rest is just regular TypeScript-shaped type algebra.
That makes memory qualification just ordinary type-level computation: userland code can introspect and rewrite ownership and placement using the same type system for any other type.
Inside a type declaration, `this` in type or static position also carries the current instantiated form of that type to query against with the `*Of` and `Is*` family. 

```ds
struct Buffer<T> {
    @if(PlaceOf<this> == "shared")
    lock: SharedLock;

    value: T;
}

declare const localBuffer: Buffer<string>;
declare const sharedBuffer: shared Buffer<string>;

sharedBuffer.lock satisfies SharedLock;
PlaceOf<typeof localBuffer> satisfies "ambient";
PlaceOf<typeof sharedBuffer> satisfies "shared";
```

# Runtime

## Modules

Like many JS/TS runtimes, Destack supports importing additional file types beyond code modules.

### Import Meta

`import.meta` exposes module and profile metadata during static and comptime evaluation.

| Field | Description | Type | Examples |
|-------|-------------|------|----------|
| `import.meta.url` | current module URL | `string` | `"file:///app/src/main.ds"`, `"https://example.com/mod.ds"` |
| `import.meta.path` | current local file path, when available | `string | undefined` | `"/app/src/main.ds"`, `undefined` |
| `import.meta.dir` | current local directory, when available | `string | undefined` | `"/app/src"`, `undefined` |
| `import.meta.emit` | output artifact format | `string` | `"js"`, `"wasm"`, `"native"` |
| `import.meta.target` | target platform and ABI | `Target` | `{ os: "linux", arch: "x64", abi: "gnu" }` |
| `import.meta.runtime` | runtime environment | `RuntimeMeta` | `{ name: "destack", version: "0.1.0" }` |
| `import.meta.debug` | debug/development build flag | `bool` | `true`, `false` |
| `import.meta.test` | test build flag | `bool` | `true`, `false` |
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
