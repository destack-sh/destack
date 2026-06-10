# Comparison

It's generally helpful to learn a new thing in contrast to things we already know, and in that spirit we have compiled a short comparison overview of what Destack looks like coming from each of these languages.
The Destack language is deliberately trying to be "TypeScript++", so that's where we begin, but there are many similarities and some differences to other well known languages - most notably Rust - that bear pointing out.

## TypeScript

**Destack is a superset of the "modern strict" subset of TypeScript**.
The intended use case for Destack is making TS-shaped code correct and optimal, which requires both cutting all accumulated dynamic magic and introducing systems-y features for memory control.
Accordingly, Destack excludes legacy syntax and all sorts of dynamic shapes and protocols that are not statically sound, and also removes a few rarely used footguns.

It's important to note that Destack deliberately has **no JavaScript or NPM interoperability**: `.ds` code cannot import or call arbitrary JS, and Destack packages cannot depend on npm packages in any way.
The main reason for this is that to take full advantage of Destack's features and full integration across the stack, we need to essentially rewrite all libraries anyway.

In general, most strict modern TypeScript needs no changes at all:

```ds
interface User {
    name: string;
    age: number;
}

function adults(users: User[]): User[] {
    return users.filter((user) => user.age >= 18);
}

const names = adults([{ name: "Ada", age: 36 }]).map((user) => user.name);
```

### Modules

Destack supports only ESM syntax: one static module graph, nothing mutable at runtime.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Type-only imports / exports** | `import type { User } from "./user"` | supported as plain `import` / `export` aliases |
| **CommonJS** | `require("x")`, `module.exports` | not supported, a legacy mutable runtime module system |
| **Namespace declarations** | `namespace Name { ... }` | not supported, use real modules |
| **String module declarations** | `declare module "pkg" { ... }` | not supported, declare real modules instead |
| **Declaration merging** | two `interface User` blocks | not supported, one declaration per name, extensions cover augmentation |
| **Import type queries** | `import("pkg").User` | not supported, use ordinary static imports |
| **Dynamic module loading** | `import(expr)` | not source-level loading, though JS output may still chunk |
| **Import defer** | `import defer * as ns from "pkg"` | not supported |
| **Import assertions** | `assert { type: "json" }` | not supported, use standardized `with { ... }` attributes |
| **String export names** | `export { value as "name" }` | not supported |
| **Circular inference** | mutually inferred module exports | not supported across modules |
| **Flow and JSDoc typing** | `/** @type {Foo} */` | ignored, or rejected when not valid TS/TS++ |

### Values

Bindings are strict-mode `const` and `let` with definite assignment, and the legacy spellings are gone.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Var declarations** | `var x` | not supported, `var` scoping is unnecessary with `const` and `let` |
| **Shadowable `undefined`** | `let undefined = value` | not supported, `undefined` is a literal keyword just like `null` |
| **Sequence expressions** | `(a, b, c)` | not supported, parenthesized comma lists are explicit tuples in `.ds` |
| **Definite assignment assertions** | `let x!: T`, `field!: T` | rejected in `.ds`, locals and fields must be initialized before use |
| **Sloppy mode** | duplicate declarations, `with` | not supported, Destack targets strict mode |
| **Callable `Symbol`** | `Symbol("name")` | not supported, use `Symbol.create("name")` |

### Primitives

The TypeScript primitives keep their meaning, but we support more scalar types.

| Feature | Example | Ruling |
| --- | --- | --- |
| **`number`** | `let x: number` | stays the default numeric type, an alias for `float64` |
| **Integer and float widths** | `int32`, `uint8`, `float32`, ... | added as real scalar types |
| **Single-quoted literals** | `'A'` | `char` in `.ds`, string in `.ts` / `.tsx` |
| **String indexing** | `text[0]` | yields `char` in `.ds`, and traps on a lone surrogate |

#### Characters

Single quotes are scalars, double quotes are strings:

```ds
const initial = 'A'; // char, not a one-character string

const text = "héllo";

text.length satisfies uint; // UTF-16 code units, as in JS
text[1] satisfies char;
```

### Arithmetic

Integer arithmetic traps on overflow in all build configurations, and `as` never loses information.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Overflow** | `int32` max `+ 1` | traps in every build mode |
| **Wrapping** | `(a + b) & 0xff` idioms | explicit: `wrappingAdd`, `Wrapping<T>` |
| **Numeric `as`** | `big as int8` | lossless conversions only, lossy ones are explicit methods |

#### Conversions

Lossy numeric conversion requires an explicit method call:

```ds
const big: int32 = 100_000;

const bad = big as int8;    // ERROR: lossy conversion
big.truncate<int8>();       // keep the low bits
big.saturate<int8>();       // clamp to bounds
big.tryInto<int8>();        // checked
```

### Unknown

`unknown` stays; `any` dies.

| Feature | Example | Ruling |
| --- | --- | --- |
| **`any`** | `let x: any` | forbidden in all sources, an unchecked escape hatch in both directions |
| **`unknown`** | `declare const data: unknown` | supported, and induces a generic in constraint positions |

#### Any and Unknown

`unknown` does the safe half of `any`'s job:

```ds
declare const input: any; // ERROR: `any` is forbidden

declare const data: unknown;

if (data is string) {
    data satisfies string;
}
```

### Shapes

Object shapes are static and exact: no prototype tricks, no runtime mutation, and fresh literals are checked everywhere.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Interchangeable `type` / `interface`** | data-shaped `interface Point` in a field | diverges in storage positions |
| **`Record<K, V>`** | `Record<string, User>` | closed utility type, use `Map<K, V>` for dynamic keyed storage |
| **Declaration expressions** | `const C = class {}` | not supported, runtime type generation is not statically knowable |
| **Prototype objects** | `.prototype`, `.__proto__`, `Object.setPrototypeOf` | not supported |
| **Shape mutation** | `delete obj.x`, `Object.defineProperty` | forbidden, object shapes must stay statically known |
| **Metaobject dispatch** | `Proxy`, most `Reflect.*` APIs | not supported |
| **Array holes** | `[1,,3]` | not supported, sequences are dense |
| **Excess properties** | fresh literal with extra fields | rejected at every boundary, not just some |

#### Type vs Interface

In TypeScript, `type` and `interface` are mostly interchangeable.
In Destack they diverge in storage positions: aliases are exact data shapes, interfaces are constraints and induce hidden generics when stored.

```ds
type Point = { x: number; y: number };

struct Rectangle {
    start: Point; // exact storage, as in TypeScript
}

interface PointLike {
    x: number;
    y: number;
}

struct Sprite {
    position: PointLike; // induces a hidden generic: Sprite<T: PointLike>
}
```

#### Freshness

The classic typo stays caught, at every boundary:

```ds
interface SquareConfig {
    color?: string;
    width?: number;
}

declare function createSquare(config: SquareConfig): void;

createSquare({ colour: "red", width: 100 }); // ERROR: excess property `colour`
```

### Classes

Classes keep their TypeScript surface but become properly nominal.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Structural classes** | same-shaped classes interchangeable | classes are nominal, structure does not substitute for declarations |
| **Private fields** | `#field` | not supported, redundant with real `private` in `.ds` |
| **Class index signatures** | `class C { [key: string]: T }` | not supported, classes have fixed declared members |

#### Nominality

Two classes with the same shape are still two classes:

```ds
class Point2 {
    x = 0;
    y = 0;
}

class Point3 {
    x = 0;
    y = 0;
    z = 0;
}

const point: Point2 = new Point3(); // ERROR (OK in TypeScript)
const plain: Point2 = { x: 0, y: 0 }; // ERROR (also OK in TypeScript!)
```

TypeScript classes only turn nominal once they have a `private` member, which is why the branding hack works; Destack classes are nominal always.

### Enums

Enum fields are nominal constants, without TypeScript's number leakage.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Enum coercion** | `Level.A` as `number` | no implicit coercion, enum fields are nominal constants |

### Generics

Generics work as in TypeScript, with explicitness required where inference would have to guess.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Declaration parameter inference** | `function f(x = 1) {}` | not supported, public declaration surfaces need explicit parameter types |
| **Generic argument ambiguity** | `Foo<{ value: string }>` | object-shaped type arguments need `type` |

### Variance

TypeScript's best known soundness hole is closed: mutable positions are invariant.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Mutable covariance** | `Circle[]` as `Shape[]` | not supported, mutable generic positions are invariant |

#### Arrays

Readonly views keep the safe half of array covariance:

```ds
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];

const shapes: Shape[] = circles;        // ERROR: writing a Square through `shapes` would corrupt `circles`
const view: readonly Shape[] = circles; // OK: readonly views are covariant
```

### Guards

Flow narrowing works as in TypeScript; only the guards themselves change.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Truthiness** | `if (value)` | boolean values only, control flow needs explicit tests |
| **Runtime `typeof` narrowing** | `typeof x === "string"` | not supported, use `is` or `instanceof` |
| **Type predicate / assertion signatures** | `value is T`, `asserts value is T` | not supported, callable type guards claim refinements that cannot be checked |

#### Truthiness

`if` takes booleans, not values:

```ds
function printAll(values: string | string[] | null): void {
    if (values) {
        // ERROR: condition must be boolean
    }

    if (values !== null) {
        // OK: same narrowing, explicit test
    }
}
```

#### Is

`is` replaces both runtime `typeof` tests and predicate signatures, and the compiler checks it instead of trusting it:

```ds
function padLeft(padding: number | string, input: string): string {
    if (padding is number) {
        return " ".repeat(padding) + input;
    }

    return padding + input;
}
```

### Operators

Operators keep their JavaScript meaning wherever JavaScript has one, and stop existing where it does not.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Loose equality coercion** | `a == b` on objects | no object coercion |
| **`===` on value types** | `pointA === pointB` | rejected for structs, tuples, and fixed arrays, use `Equal` |
| **Coercion hooks** | `valueOf`, `Symbol.toPrimitive` | not used for implicit coercion |
| **Symbol magic** | `Symbol.hasInstance`, `Symbol.species` | not supported, use typed protocols |
| **Non-null assertions** | `value!` | supported as `Try` / must unwrapping, an explicit runtime operation |
| **Type angle assertions** | `<T>value` | not supported, use `value as T` or `value satisfies T` |

#### Equality

Value types compare with `Equal`, not identity:

```ds
struct Point {
    x: int32;
    y: int32;
}

Point { x: 1, y: 2 } === Point { x: 1, y: 2 }; // ERROR: `===` has no JS meaning for value types
```

### Closures

Callable context is explicit: no ambient `arguments`, no dynamic `this` rebinding.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Ambient call metadata** | `arguments`, `new.target`, dynamic `this` | not supported as magic bindings, callable context is explicit |
| **Ambiguous generic arrow** | `<T>() => value` | not supported, ambiguous with TSX, use `<T,>() => value` |
| **Dynamic constructors** | `new (factory())()` | not supported, construct values with type syntax like `new Widget<T>()` |

### Loops

Every loop form works unchanged; only the manual iteration protocol is replaced.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Manual iterator protocol** | `iter.next().done` | not supported, iteration is the nominal `Iterator` protocol, while `for-of` and spread lower unchanged |

### Trees

TSX works as tree syntax, minus XML namespace semantics.

| Feature | Example | Ruling |
| --- | --- | --- |
| **XML namespace resolution** | `<svg:path />` | no `xmlns` binding semantics, namespaced tags are intrinsic string tag names |

### Errors

Failures are `Result` values: `throw` is gone, and a `Promise` never rejects.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Exceptions** | executing `throw` | not supported in native Destack code |
| **`try` / `catch` / `finally`** | `try { ... } catch (e) { ... }` | works, as sugar over `Result` control flow |
| **Promise rejection** | `.catch`, `Promise.reject` | gone, a `Promise<T>` always fulfills |
| **Rejection-shaped APIs** | `Promise.any`, `allSettled`, two-arg `then` | gone with rejection |
| **Floating promises** | bare `refresh();` statement | denied by default, await it, return it, or hand it to a scope |
| **Thenables** | `await customThenable` | not supported, `await` works on the well known `Promise<T>` only |

#### Exceptions

The consuming side still reads like TypeScript:

```ds
newtype ParseError = string;

declare function parsePort(raw: string): Result<uint16, ParseError>;

try {
    const port = parsePort(input)?;
    connect(port);
} catch (error: ParseError) {
    report(error);
}
```

#### Promises

Async failure travels as `AsyncResult<T, E>`, which is just `Promise<Result<T, E>>`:

```ds
declare function load(): Promise<User>;

load().catch(report); // ERROR: `catch` does not exist, promises do not reject

declare function fetchUser(id: string): AsyncResult<User, LoadError>;

async function rename(id: string, name: string): AsyncResult<User, LoadError> {
    const user = (await fetchUser(id))?;
    return Result.ok(user.with({ name }));
}
```

### Decorators

Decorators stay, and move to compile time: a decorator is a `comptime` value that may transform its target as a [macro](DESIGN.md#macros).

| Feature | Example | Ruling |
| --- | --- | --- |
| **Class and member decorators** | `@route("/users") class ... ` | supported, evaluated at compile time |
| **Runtime decorator metadata** | `emitDecoratorMetadata` | not supported, use [reflection](DESIGN.md#reflection) |

### Comptime

Runtime code generation conflicts with ahead-of-time compilation, so generation moves to compile time.

| Feature | Example | Ruling |
| --- | --- | --- |
| **Dynamic code generation** | runtime `eval`, `new Function` | not supported except explicit `comptime eval` |

## Rust

Destack's memory model is Rust-shaped with inverted defaults: values are managed unless you opt into ownership, and mutability is decoupled from exclusivity.

### Ownership

#### Defaults

In Rust every value has exactly one owner.
In Destack the same is true, but for managed values the owner is the runtime, and ownership in the Rust sense is opt-in with `^T`:

```ds
let user = new User();         // managed: the runtime owns it
let owned: ^User = new User(); // owned: single owner, deterministic drop
```

The managed default is why existing TypeScript works unchanged.

#### Borrowing

Unlike in Rust, mutability is decoupled from borrowing: there are three borrow forms, and the aliased mutable `&T` sits between Rust's two.

```rust
let mut point = Point { x: 1, y: 2 };
let a = &mut point;
let b = &mut point; // ERROR: cannot borrow `point` as mutable more than once
```

```ds
let point = ^Point { x: 1, y: 2 };
let a = &point;
let b = &point; // OK: mutable borrows may alias

a.x = 3;
b.y = 4;
```

This is sound because a Worker is a single-threaded execution domain with explicit suspension points, so the writes cannot race.
`&exclusive` is `&mut`, and it still exists for when we do want no aliasing:

```ds
let c = &exclusive point; // ERROR: `a` and `b` are still live
```

Interior mutability (`Cell`, `RefCell`) exists too, but aliased `&T` covers most of what Rust needs it for.

#### Exclusivity

Exclusivity is reserved for operations that may invalidate another borrow, instead of being required for every write.
Where invalidation is possible, Destack and Rust agree:

```rust
let mut items = vec![1, 2, 3];
let first = &items[1];
items.push(4); // ERROR: cannot borrow `items` as mutable
first;
```

```ds
let items: Array<int32> = [1, 2, 3];
let item = &readonly items[1];

items.push(4); // ERROR: growth needs exclusive access while `item` is live
item;
```

#### Moves

Owned values move exactly like Rust values; managed handles copy freely.

```ds
let a: ^Buffer = Buffer.open();
let b = a;
a.length; // ERROR: `a` moved into `b`

let x = new User();
let y = x;
x.name; // OK: managed handles alias
```

#### Drop

`Drop` exists with the same meaning: owned values run their finalizer deterministically when their lifetime ends, and managed values run it when the garbage collector reclaims them.

```rust
impl Drop for Connection {
    fn drop(&mut self) { self.close(); }
}
```

```ds
class Connection implements Drop {
    drop(): void {
        this.close();
    }
}
```

### Lifetimes

#### Inference

Lifetimes are ordinary `comptime` value parameters (`<comptime L: Lifetime>` on `Borrowed<T, L>`), and they are inferred everywhere, including from function bodies.

```rust
fn first<'a>(items: &'a [String]) -> &'a str {
    &items[0]
}
```

```ds
function first(items: &[string]): &string {
    &items[0]
}
```

#### Structs

Borrow-holding structs work like Rust's, with the lifetime as a `comptime` parameter:

```rust
struct View<'a> {
    name: &'a str,
}
```

```ds
struct View<comptime L: Lifetime> {
    name: Borrowed<string, L>;
}
```

#### Declarations

The one place lifetimes are spelled out is `declare` signatures, because there is no body to infer from.
They participate in regular type algebra, including unions:

```ds
declare function choose<comptime L1: Lifetime, comptime L2: Lifetime>(
    first: Borrowed<string, L1>,
    second: Borrowed<string, L2>,
): Borrowed<string, L1 | L2>;
```

### Traits

#### Interfaces

Traits map to `newtype interface`s.
The standard `Iterator` is deliberately the same shape:

```rust
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

```ds
newtype interface Iterator {
    type Item;
    next(): Option<this.Item>;
}
```

#### Implementations

There is no orphan rule: any module may implement any interface for any type, because Destack compiles whole programs and checks uniqueness globally.

```rust
// ERROR: only traits defined in the current crate can be implemented (E0117)
impl Display for Vec<u8> { ... }
```

```ds
// fine: `implements` is globally unique per (type, interface) pair
extension of Array<uint8> implements Show {
    show(): string {
        `${this.length} bytes`
    }
}
```

Conflicts between packages are whole-program compile errors, and libraries should only implement pairs they own one side of (a default-`warn` diagnostic nudges accordingly).

#### Blankets

Blanket implementations are supported with one familiar restriction: an `implements` blanket over a bare bounded parameter may only come from the package that declares the interface.

```rust
impl<T: Display> Pretty for T { ... } // only valid in the crate that owns Pretty
```

```ds
extension<T: Show> of T implements Pretty { ... } // same rule, same reason
```

Blankets over nominal applications (`extension<T> of Box<T> implements Show`) are open to anyone, which Rust's orphan rule forbids.

#### Associated Types

Associated types and constants work as in Rust, with positional refinement as sugar:

```rust
fn sum(values: impl Iterator<Item = u8>) -> u32 { ... }
```

```ds
function sum(values: Iterator<uint8>): uint32 { ... } // sugar for Iterator<type Item = uint8>
```

### Data

#### Enums

Rust enums map to `newtype` unions over object variants, with `@derive(Tagged)` supplying constructors and discriminants:

```rust
enum Shape {
    Circle { radius: f64 },
    Square { side: f64 },
}
```

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "circle"; radius: float64 }
    | { kind: "square"; side: float64 };

match (shape) {
    Shape.Circle({ radius }) => radius * radius * 3.14
    Shape.Square({ side }) => side * side
}
```

#### Option

`Option<T>` is the nominal carrier over `T | null`, so it interoperates with TypeScript's nullable style instead of replacing it:

```ds
const some: Option<int32> = 1;
const none: Option<int32> = null;

const raw: int32 | null = some; // projects back to the nullable union
```

### Errors

#### Results

`Result<T, E>` and `?` carry over directly, and `try` / `catch` is sugar over the same control flow:

```rust
let user = load_user(id)?;
```

```ds
const user = loadUser(id)?;
```

#### Panics

Panics take string messages only, and there is no `catch_unwind`: the Worker is the fault boundary, and the supervising parent observes the exit.

```rust
let result = std::panic::catch_unwind(|| risky()); // recoverable in-process
```

```ds
panic("invariant broken"); // unwinds the Worker; the parent sees WorkerExit "panicked"
```

### Concurrency

#### Send and Sync

`Send` and `Sync` exist with the same meaning and the same structural derivation, applied per memory form:

| Form | Crosses Workers when |
| --- | --- |
| local managed `T` | never |
| `^T` | `T: Send` |
| `&readonly T` | `T: Sync`, owned or static source |
| `&exclusive T` | `T: Send`, owned or static source |
| `&T` | never |

The `&T` row is the cost of aliased mutability: it is only sound within one Worker, so it never crosses.

#### Tasks

Where `tokio::spawn` hands back a detachable `JoinHandle`, spawned work here is owned by a scope that cannot exit while children are pending:

```rust
let handle = tokio::spawn(async { fetch().await }); // leaks if never awaited
```

```ds
await using scope = TaskScope.open();
const task = scope.spawn(() => fetch()); // owned: cancelled or joined before the scope exits
```

#### Workers

Threads map to Workers: isolated heaps, explicit `shared` memory, and message passing.
Borrows of owned data may cross into scoped child tasks (subject to `Send`), which gives fork-join parallelism over borrowed data without `Arc` ceremony.

### Generics

#### Monomorphization

Both languages monomorphize, and Destack additionally induces generics implicitly wherever a constraint appears in a parameter, so `impl Trait` maps to nothing at all:

```rust
fn draw(shape: impl Shape) { ... }     // argument position: universal
fn make() -> impl Shape { ... }        // return position: existential
```

```ds
function draw(shape: Shape): void {}   // same universal, no keyword
function make(): Shape { ... }         // same existential, no keyword
```

#### Constants

Const generics map to `comptime` parameters, and value constraints are spelled as interval bounds or checked with `comptime assert` at instantiation.

```rust
struct Buffer<const N: usize> { data: [u8; N] }
```

```ds
struct Buffer<comptime N: 0..=4096> {
    data: [uint8; N];
}
```

#### Erasure

`dyn Trait` maps to `Dynamic<T>`, with `DynamicSafe` mirroring object safety:

```rust
let shapes: Vec<Box<dyn Shape>> = vec![circle, square];
```

```ds
let shapes: Dynamic<Shape>[] = [circle, square];
```

#### Variance

Rust infers variance and never surfaces it, while Destack computes it the same way but lets declarations pin it with `in` / `out`, checked against usage like TypeScript.

### Unsafe

`@unsafe` plays the same role as `unsafe`, with the same culture: raw pointers are safe to create and carry, and only dereferencing them needs the marker.

```rust
let pointer = &user as *const User;
unsafe { (*pointer).name.clone() }
```

```ds
let pointer: *User = &user;

@unsafe
function read(pointer: *User): string {
    pointer.name.clone()
}
```

### Comptime

#### Macros

Rust splits metaprogramming across `macro_rules!`, proc macros, and derive macros, each with its own toolchain.
Destack has one mechanism: a decorator is a static term, and one that implements `Macro` may transform its target as a typed declaration using the well known `eval` function.

```rust
#[derive(Serialize)]
struct User { name: String }
```

```ds
@derive(Serialize)
struct User {
    name: string;
}
```

Generated code comes from `comptime eval` over reflected declarations, so macros compose with ordinary functions and expansion runs to a fixed point.
In this example the `derive` can usually even be emitted since Destack additionally supports auto-derive for builtin derive macros.

#### Const Functions

`const fn` maps to `comptime` evaluation of ordinary functions: anything the compiler can run is fair game, with a budget instead of a stability whitelist.

```rust
const SIZE: usize = compute_size(); // compute_size must be a const fn
```

```ds
const SIZE: usize = comptime computeSize(); // any function, evaluated during compilation
```

## Flow

Flow tried sound-by-default inside TypeScript-shaped syntax first, and Destack lands on many of the same answers.

### Objects

#### Exactness

Flow made object types exact by default; Destack gets the same rejections through freshness and excess property checks:

```flow
function send(user: { name: string }) {}

send({ name: "Ada", admin: true }); // ERROR: `admin` is missing in object type
```

```ds
function send(user: { name: string }): void {}

send({ name: "Ada", admin: true }); // ERROR: excess property `admin`
```

### Nominality

#### Classes

Both Flow and Destack make classes nominal, against TypeScript's structural treatment:

```ds
class Left {
    value: int32 = 0;
}

class Right {
    value: int32 = 0;
}

const value: Left = new Right(); // ERROR in Flow and Destack, OK in TypeScript
```

#### Opaque Types

Flow's `opaque type` is the closest thing in the TS family to `newtype`, with one difference: Flow's opacity ends at the module boundary, while a `newtype` is nominal everywhere and constructed explicitly.

```flow
export opaque type ID = string; // transparent inside this file, opaque outside
```

```ds
export newtype Id = string; // nominal everywhere, constructed as Id("...")
```

### Variance

#### Sigils

Flow annotates variance per property with `+` / `-`; Destack computes it from the member surface, and `in` / `out` exist only as checked assertions:

```flow
type Source<+T> = { +value: T };
```

```ds
interface Source<T> {
    readonly value: T; // computes covariant on its own
}
```

### Predicates

#### Guards

Flow's `%checks` functions assert refinements; Destack's `is` expressions are guards the compiler understands directly:

```flow
function isString(value: mixed): boolean %checks {
    return typeof value === "string";
}
```

```ds
if (value is string) {
    value satisfies string;
}
```

## AssemblyScript

AssemblyScript is the closest cousin: TypeScript syntax compiled ahead-of-time with explicit numeric types.
Its gaps show what a TS-shaped native language cannot afford to give up.

### Numbers

#### Naming

The numeric types correspond one-to-one, spelled out so they read as TypeScript:

```ts
function clamp(value: i32, low: i32, high: i32): i32 { ... }
```

```ds
function clamp(value: int32, low: int32, high: int32): int32 { ... }
```

`number` stays an alias for `float64`, so unannotated TypeScript keeps its meaning.

#### Overflow

AssemblyScript wraps silently, matching WASM; Destack traps on overflow in every build mode, and wrapping is explicit:

```ts
let big: i32 = 2147483647;
big + 1; // -2147483648, silently
```

```ds
let big: int32 = 2147483647;
big + 1;                 // traps: overflow
big.wrappingAdd(1);      // -2147483648, explicitly
Wrapping(big) + 1;       // or wrap by type
```

### Expressiveness

#### Closures

Closures over mutable locals - the everyday TypeScript AssemblyScript cannot compile - work natively:

```ds
function counter(): () => int32 {
    let count = 0;

    return () => {
        count += 1;
        count
    };
}
```

#### Unions

Union types, the other major gap, are first-class and reified:

```ds
function describe(value: int32 | string): string {
    value is string ? value : `#${value}`
}
```

### Memory

#### Values

`@unmanaged` classes map to `struct`: value types are a declaration form, not an annotation on classes.

```ts
@unmanaged class Vec2 { x: f32; y: f32; }
```

```ds
struct Vec2 {
    x: float32;
    y: float32;
}
```

#### Equality

AssemblyScript's `==` / `===` split confused users badly enough that they changed it.
Destack keeps `===` only where JavaScript gives it a meaning (primitives and managed identity) and rejects it elsewhere, so the question never comes up for value types.

## C#

C# is the other Hejlsberg language, and several of its core distinctions transfer almost verbatim.

### Values

#### Structs and Classes

The `struct` / `class` split means the same thing: values copied inline versus managed references.

```csharp
struct Point { public double X, Y; }

class User { public string Name = ""; }
```

```ds
struct Point {
    x: float64;
    y: float64;
}

class User {
    name: string = "";
}
```

#### Records

Records map to structs for value semantics or newtypes for nominal wrappers, with equality via `@derive(Equal)` instead of being implicit:

```csharp
record Point(double X, double Y);
```

```ds
@derive(Equal)
struct Point {
    x: float64;
    y: float64;
}
```

(Though again it should be noted that Destack suports auto-deriving for builtin derives, so equality would be derived automatically here.)

#### Properties

C# properties map to accessors directly:

```csharp
class Counter {
    private int value;
    public int Count {
        get => value;
        set => this.value = value;
    }
}
```

```ds
class Counter {
    private value: int32 = 0;

    get count(): int32 {
        this.value
    }

    set count(next: int32) {
        this.value = next;
    }
}
```

### Generics

#### Variance

The `in` / `out` spelling is C#'s, with one difference: variance is computed from usage, and the annotations are checked assertions instead of requirements.

```csharp
interface ISource<out T> { T Take(); }
```

```ds
interface Source<out T> {
    take(): T;
}
```

#### Constraints

`where` clauses are nearly character-identical:

```csharp
T Largest<T>(List<T> values) where T : IComparable<T> { ... }
```

```ds
function largest<T>(values: T[]): T where T: Comparable<T> { ... }
```

### Errors

#### Exceptions

C# keeps exceptions; Destack moves failure into values like Rust:

```csharp
try {
    var user = await LoadUser(id);
} catch (LoadException error) {
    Report(error);
}
```

```ds
const result = await loadUser(id);

match (result) {
    Ok { value } => use(value)
    Err { error } => report(error)
}
```

#### Nullability

C#'s nullable reference types map to plain nullable unions, checked the same way:

```csharp
string? name = FindName(id);
if (name is not null) { Use(name); }
```

```ds
const name: string | null = findName(id);

if (name !== null) {
    use(name);
}
```

### Concurrency

#### Async

`async` / `await` carries over unchanged, with one naming collision to know about: C#'s `Task<T>` is Destack's `Promise<T>` (the awaitable value), while Destack's `Task<T>` is a scope-owned work item closer to a structured `Task.Run`.

```csharp
async Task<User> LoadUser(string id) { ... }
```

```ds
async function loadUser(id: string): Promise<User> { ... }
```
