# Destack Language Specification

> **The Destack language is a spiritual "TypeScript++" for building optimal, correct, integrated software systems across the _full_ stack.**
>
> This document describes the syntax and semantics of **`.ds` files**, how JS/TS features work, and what exactly the ++ parts are and how they interact with the rest of the language.
> However, this document is a reference and _not_ a complete or formal language or runtime specification, and is not intended to be.
> Unless otherwise specified, we follow TypeScript semantics.

See [DESIGN](DESIGN.md) for the high-level design principles and objectives.
See [COMPATIBILITY](COMPATIBILITY.md) for the full file type matrix and exclusions.

## Literals

Destack supports all JS/TS literals, as well as some additions.
See [Types](#types) for literal type behavior and assignability rules.
See [Trees](#trees) for tree literal syntax and routing rules.

### Numeric Literals

Numeric literals work just like in JS/TS:

```ds
42                   // integer (inferred precision)
42n                  // bigint (TypeScript)
3.14                 // float (inferred as float64)
0x1A                 // hexadecimal
0o17                 // octal
0b1010               // binary
1_000_000            // numeric separators
```

### String Literals

String literals work exactly like in TypeScript, with the addition that single-quoted literals are recommended for single-character "strings" (Rust-style):

```ds
"hello"              // double-quoted string
'a'                  // character (single Unicode codepoint)
```

### Template Literals

Template literals support interpolation and can use tags, just like in JS/TS:

```ds
`hello`                           // simple template
`hello ${name}`                   // interpolated template
`${a} + ${b} = ${a + b}`          // multiple interpolations
sql`SELECT * FROM users`          // tagged template
html`<div>${content}</div>`       // tagged template
```

Tagged templates call a function with the string parts and interpolated values, enabling DSLs for SQL, HTML, CSS, GraphQL, and more (just like in JS/TS).

### Regex Literals

Regex literals work exactly like JS/TS:

```ds
/pattern/            // regex
/\d+/g               // regex with flags
/hello\s+world/i     // case insensitive
```

### Collection Literals

Array and object literals work like TypeScript:

```ds
[1, 2, 3];             // array
{ a: 1, b: 2 };        // anonymous object literal
```

Destack adds tuples:

```ds
(1, 2, 3);             // tuple
();                    // empty tuple
```

## Types

Destack extends TypeScript's type system with tuples, precise primitives, explicit nominality, extension methods, broader compile time value spaces, and where clauses.
See [Ownership](#ownership) for explicit ownership and borrow semantics.
See [Declarations](#declarations) for declaration forms that introduce type and value symbols.

**TSC, inference and TypeScript pain**:
Destack tries very hard to be TSC faithful and support the full modern TS feature set.
Out of necessity and some strong opinions, Destack is stricter in some places, and we try very hard to make modern TS code _just work_.
However, Destack is a completely different from-scratch compiler architecture, and there are some almost unavoidable differences in inference ordering and capabilities in gnarly edge cases.

### Primitives

TypeScript has `number`, `string`, `boolean`, `bigint`, `symbol`, `null`, `undefined`, and `void`.
Destack adds precise numeric types while keeping the originals as aliases.

#### Special Types

The "special" types work like in TypeScript, with Destack extending the receiver-aware `this` type.

- `void` - empty type (no value)
- `null` - explicit zero/unset value
- `undefined` - uninitialized value
- `never` - bottom type (unreachable)
- `any` - top type (discouraged, forbidden in strict mode)
- `unknown` - explicit top type
- `this` - receiver type (see below)

#### This

Destack supports TypeScript's polymorphic `this` type for instance members, and extends it to static type positions.
`this` is only valid in type positions and resolves based on the surrounding declaration:
- In instance members, `this` resolves to the concrete receiver type.
- In static type positions (like static members or static arguments), `this` resolves to the containing type itself.

#### Booleans

Booleans work unchanged, though Destack requires all boolean values to be actual booleans (explicit casts are required, like in ESLint's `no-implicit-coercion`)

#### Numbers

TypeScript uses just `number` for all numerics, which is an IEEE 64-bit `float64`.
Destack further adds more precise integer types, including for arbitrary width (inspired by Zig):
- `int8`, `int16`, `int32`, `int64`, `int128`, and any `int<width>` like `int17` (signed)
- `uint8`, `uint16`, `uint32`, `uint64`, `uint128`, and any `uint<width>` like `uint17` (unsigned)
- `isize`, `usize` - pointer-sized integers
- `int` / `uint` - default integer width for the current compiler configuration

Destack also supports specifying `float` explicitly:
- `float32`, `float64`
- `float` - default float width for the current compiler configuration
- `number` - JS-compatible numeric supertype that accepts precise ints/floats

Destack's precise integers behave like real machine integers with defined overflow semantics (wrapping, saturating, or trapping), proper bitwise operations.

Assignments from `number` to a precise numeric type (`int32`, `float64`, and so on) require an explicit conversion:
 - Explicit float to integer casts are checked at runtime.
 - Finite values in range truncate toward zero.
 - NaN or out of range values trap.
 - Unsigned targets treat negative values as out of range.
 - Saturating float to integer intrinsics clamp to bounds and map NaN to 0.

#### Characters and Strings

In addition `string`, Destack supports a single `character`:
- `string` - UTF-8 string (same as TypeScript)
- `character` - single Unicode codepoint

Character and string are distinct types, and characters are not implicitly assignable to strings.

#### Template Literal Types

Template literal types use the same backtick syntax in type positions, like in TypeScript.
Spans inside `${...}` are type expressions that must be stringifiable:
- `string`, `number`, `bigint`, `boolean`, `null`, `undefined`, `any`
- Template literal types and unions of stringifiable types

Template literal type assignability matches TypeScript's rules:
- A string literal is assignable to a template literal type when it matches the literal parts and each span constraint.
- A template literal type is assignable to `string`.
- Spans of `never` accept no strings, so template literals containing `never` are uninhabited.

### Type Aliases and Newtypes

Type aliases are transparent wrappers to another type like in TypeScript.
Destack also adds a `newtype` for proper nominal types:

```ds
type Point = { x: float32, y: float32 };    // structural (TypeScript)
newtype SpecialPoint = Point;                     // nominal (distinct type)
```

A `newtype` creates a distinct type that structurally assignable types are not assignable to.
`UserId` and `OrderId` won't mix even if both are `int64`.
Newtypes can wrap scalars, tuples, or objects:

```ds
newtype UserId = int64;                    // wraps scalar
newtype Point = (float32, float32);        // wraps tuple
newtype Config = { debug: boolean };       // wraps object
```

Construction syntax matches the underlying type:
```ds
const id = UserId(42);                     // scalar: Name(value)
const p = Point(1.0, 2.0);                 // tuple: Name(elements...)
const c = Config { debug: true };          // struct: Name { fields... }
```

Pattern matching / unwrapping also works with newtype constructors:
```ds
match (id) {
    UserId(0) => "system"
    UserId(n) => `user ${n}`
}
```

All nominal types, including newtypes, can receive methods via `extension`s (see Extensions below).
The same mechanism works for all types: structs, enums, newtypes, even foreign types and builtin primitives like `int32` or `Date`.

### Unions and Intersections

Structural combinator types work like in TypeScript:

```ds
int32 | string | null      // union
A & B                      // intersection
```

#### Discriminated Unions

Destack supports TypeScript-style discriminated unions:

```ds
type Result<T, E> =
    | { kind: 'ok', value: T }
    | { kind: 'err', error: E }

function divide(a: int, b: int): Result<int, string> {
    if (b == 0) {
        { kind: 'err', error: "division by zero" }
    } else {
        { kind: 'ok', value: a / b }
    }
}
```

### Arrays and Tuples

Destack supports dynamic arrays, of course, but also statically-sized arrays:
```ds
int32[]                    // dynamic array
readonly int32[]           // readonly dynamic array
int32[N]                   // fixed-size array
readonly int32[N]          // readonly fixed-size array
```

<!-- FUGU #Cleanup -->
Supporting fixed sized arrays with this clean syntax is very nice, but unfortunately overloads T[N] with TypeScript's type indexing.
So, `T[N]` has two meanings in Destack: indexed access and fixed-size arrays:
- In `.ts` and `.d.ts`, `T[N]` always uses TypeScript indexed-access semantics.
- The builtin alias `FixedArray<T, comptime N>` is exactly `T[N as comptime]`.
- In `.ds`, `T[N as comptime]` always means fixed-size array construction.
- In `.ds`, if `N` is type-space, `T[N]` is indexed access.
- In `.ds`, otherwise we check indexed-access admissibility for `T[N]` using normal type-index rules.
- If indexed access is admissible, `T[N]` is indexed access.
- This includes numeric literals and concrete comptime values, for example `string[4]`.
- If indexed access is inadmissible and `N` resolves to a static integer value, `T[N]` is fixed-size array.
- In ambiguous value-space cases where fixed-size semantics are intended, use `N as comptime`.

Destack's rules for arrays (and tuples) center around correctness and performance.
As with most other design decisions, if you're writing modern TypeScript, this will work fine.
Relatedly, Destack does _not_ support holes in arrays (we parse them, but reject at analyze time).
Array access is bounds checked and `array[n]` into `T[]` returns `T` directly.

```ds
const arrayDynamic: number[] = [1, 2, 3];
arrayDynamic[0] satisfies number;
arrayDymamic[3]; // ERROR at runtime

const arrayFixed: number[3] = [1, 2, 3];
arrayFixed[0] satisfies number;
arrayFixed[3]; // ERROR at compile time
```

Fixed-size arrays are assignable to dynamic arrays when their element types are compatible.
Tuples are fixed-length value types and are assignable to arrays when their element types are compatible.

### Dynamic Parameterisation

Functions and methods work exactly like in JS/TS.
Function calls use positional arguments:

```ds
function myFunction(a: int, b: int) {
    ...
}

myFunction(2, 3);
```

Argument types are checked against the declared parameter types.
Parameters with defaults are optional at the call site.

### Static Parameterisation ("Generics")

Static parameters extend TypeScript-style generics with explicit `comptime` value parameters.
Value parameters must be explicitly marked with `comptime` in the static parameter list; type annotations alone do not make a parameter a value parameter. (See Comptime)

Static value inference is limited and local:
- Static value arguments may be inferred from literal argument expressions in the current module when no explicit static argument is provided.
- Inference only uses fully static literal expressions: scalar literals, enum members, tuples, arrays, and objects.
- Literal array arguments may infer lengths when matching fixed-size array types like `T[N]`.
- Tuple literal arguments use Destack's tuple syntax `(a, b)` for inference.

For example, we can infer the size of an array from a literal argument:

```ds
type Buffer<comptime N: number> = uint8[N];

declare function make<comptime N: number>(value: uint8[N]): Buffer<N>;

let value = make([1, 2, 3, 4]);
value satisfies uint8[4];
```

Static value arguments may use any static expression form, as long as the resulting static value satisfies the declared value type.
Static value arguments may reference `const` bindings with static initializers, including imported constants.

Static parameterisation for types works like in TypeScript.
In Destack, static parameters also work for compile-time values via `comptime` value parameters.
TypeScript syntax with `T extends U` remains supported for type parameters.

```ds
struct Container<T: any> { // works like T extends any
    value: T
}

function identity<T>(x: T): T {
    x
}
```

Value parameters for compile-time constants:

```ds
function compute<comptime Flag: boolean>(data: uint8[]) {
    if Flag {
        ...
    }

    ...
}

compute<true>(); // pass the static argument positionally
```

Value parameters can drive type construction:

```ds
type Buffer<comptime N: number> = uint8[N];

declare let value: Buffer<4>;
value satisfies uint8[4];
```

### Variance Annotations

Type parameters may be annotated with `in` or `out` to declare variance.
`out` marks a parameter as covariant and `in` marks a parameter as contravariant, exactly like in TypeScript.

```ds
interface Producer<out T> {
    get(): T;
}

interface Consumer<in T> {
    put(value: T): void;
}

type Mapper<in T, out U> = (value: T) => U;

class Box<out T> {
    value: T;
}
```

### Where Clauses

Destack adds `where` clauses for type constraints beyond TypeScript's inline syntax.
Each clause is a type constraint of the form `Name: Type`:

```ds
function process<T>(x: T): T where T: Copy {
    // ...
}

function merge<T, U>(): T where (
    T: Mergeable,
    U: Comparable
) {
    // ...
}
```

## Declarations

Declaration forms in Destack match TypeScript, with the addition of richer static parameterisation and our concept of nominal typing (like with `newtype` behavior for `enum`s).
See [Dispatch](#dispatch) for overload and dynamic resolution rules.
See [Operators](#operators) for operator interface declarations and desugaring.

### Interface

Interfaces work like TypeScript, with optional default functions and properties:

```ds
interface Drawable {
    draw(): void;

    isVisible(): boolean {
        true  // default implementation
    }
}

interface Container<T> extends Iterable<T> {
    static Empty: this;
    size(): uint64;
    get(index: uint64): T | undefined;
}
```

#### Nominal Interfaces

Destack adds **nominal interfaces** using the `newtype interface` syntax.
Nominal interfaces require explicit ("nominal") `implements` declarations.

```ds
// structural interface (standard TypeScript behavior)
interface Drawable {
    draw(): void;
}
const x: Drawable = {
    draw() { };
};  // OK: structural match

// nominal interface (requires explicit "nominal" `implements`)
newtype interface Add<T, R = this> {
    add(other: T): R;
}

struct Vec2 {
    x: float;
    y: float;
}
const v: Add<Vec2> = Vec2 { x: 1, y: 2 }  // ERROR: Vec2 doesn't implement Add

// explicit opt-in required
extension of Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 {
        return Vec2 { x: this.x + other.x, y: this.y + other.y };
    }
}
const v: Add<Vec2> = Vec2 { x: 1, y: 2 }  // OK: Vec2 implements Add
```

Nominal interfaces are used for:
- **Operator interfaces** (`Add`, `Compare`, `Equal`, etc.) to prevent accidental operator overloading
- **Capability traits** (`Send`, `Sync`, `Copy`) for compile-time capabilities

Ordinary interfaces may also define explicit protocols such as `Clone`.

The `newtype` modifier follows the same pattern as `newtype` on type aliases—it makes the interface nominal.
Extending a nominal interface produces a nominal interface (nominality is inherited).

### Class

Classes work like in TypeScript: reference types with identity and prototype-based inheritance.
(But, as discussed, no prototype chain black magic. Just static class layouts.)

```ds
class MyClass {
    field: int32;

    constructor(value: int32) {
        this.field = value;
    }
}
```

Classes and structs can declare a `constructor` method, just like TypeScript.
Classes have **identity**: two instances are only `===` if they're the same managed object:

```ds
const a = new MyClass(1);
const b = new MyClass(1);
a == b;    // false: different instances (unless Equal implemented)
a === b;   // false: different instances
a === a;   // true: same instance
```

This is the key difference from structs—see the comparison table below.

### Struct

Destack adds `struct` for nominal value types with fixed layout.
Structs have no reference identity and no inheritance.
Structs are data with a name.
Struct declarations require a name and cannot be anonymous.

```ds
struct Point {
    x: float32;
    y: float32;
}
```

#### Struct vs Class

| | struct | class |
|---|---|---|
| Reference identity | No (`===` is error) | Yes (`===` compares managed identity) |
| Inheritance | No (use embedding) | Yes (`extends`) |
| Default passing | Value | Reference |
| Default storage | Inline | Managed reference |
| JS output | Plain object | ES6 class |

Structural object types (`type X = { ... }` and inline `{ ... }`) are reference types with identity.
Type aliases inherit the semantics of the underlying type.
Struct values can be boxed when a reference type is required.

#### Equality and Identity

Structs have no reference identity, so two structs with the same properties are equal by value.
Structs auto-derive `Equal` (field-by-field comparison) by default:

```ds
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
p1 == p2;   // true: same fields = equal (auto-derived Equal)
p1 === p2;  // ERROR: === requires reference identity, structs have none
```

Since structs have no reference identity, `===` and `!==` are compile errors on struct types.
Use `==` for value comparison.

#### Construction

Structs are nominal, so they must be explicitly constructed:
Tooling may lint `new` on structs in favor of `Point { ... }`.

```ds
let p: Point = Point { x: 1, y: 2 };  // ok: explicit construction
let p: Point = new Point(1, 2);       // ok: constructor syntax
let p: Point = { x: 1, y: 2 };        // ERROR: object literal is not Point
```

#### Pattern Matching

Struct patterns require the type name (unlike newtypes which auto-unwrap):

```ds
match (point) {
    Point { x: 0, y: 0 } => "origin";
    Point { x, y } => `at ${x}, ${y}`;
    { x, y } => ...;  // ERROR: structural pattern on nominal type
}
```

#### Interfaces and Composition

Structs can `implements` interfaces but cannot `extends` (use embedding instead):

```ds
struct Point implements Drawable {
    x: float32;
    y: float32;

    draw(): void { ... }
}

struct Transform {
    position: Vec3;
    rotation: Quat;
}

struct Player {
    ...Transform;    // embeds Transform's fields (composition)
    health: int;
}
```

#### Associated Types

Class shaped types like structs, classes, and interfaces can declare associated type aliases ("static type members") using the `type` keyword:

```ds
struct Container<T> {
    type Item = T;
    type Iter = ContainerIterator<T>;

    items: T[],

    iter(): Iter {
        ContainerIterator { items: this.items }
    }
}
```

Associated types are inherently static because they belong to the type itself, not to instances.
They can also be accessed via the containing type:

```ds
const item: Container<int>.Item = 42;  // Item resolves to int
```

Associated type declarations can include constraints and defaults:

```ds
struct SizedBox {
    type Item: number = int32;
    value: Item;
}
```

Structs and classes must provide a concrete associated type definition.
Interfaces may declare abstract associated types by omitting the default.
Interface defaults are used when an implementor does not provide a definition.

```ds
interface Iterable<T> {
    type Item;                    // abstract: implementors must provide
    type Iter: Iterator<Item>;    // abstract with constraint

    iter(): Iter;
}

extension<T> of Container<T> implements Iterable<T> {
    type Item = T;
    type Iter = ContainerIterator<T>;

    iter(): Iter { ... }
}
```

Associated type definitions must satisfy any declared constraint.
Implementors must provide compatible definitions for every abstract associated type.
Implementors may also override default associated types as long as the new definition satisfies the constraint.

##### Generic Associated Types (GATs)

Associated types can declare their own static parameters, i.e., generic associated types (GATs).

```ds
interface Slice<T> {
    type View<U>;
}

struct Buffer<T> {
    type View<U> = BufferView<T, U>;
}
```

```ds
struct Matrix<comptime Rows: uint, comptime Cols: uint> {
    type Row = float64[Cols];
    type View<comptime R: uint> = float64[R][Cols];
}
```

Associated type parameters can include constraints and defaults just like other static parameters.
Implementor associated type parameters must accept all arguments that satisfy the interface constraints.
Associated type parameters can also include comptime static value parameters.

##### Associated Type Projections

Associated types are accessed through the containing type using a projection.
Projection forms are `TypeName.Associated` or `TypeName.Associated<Args>` for generic associated types.

```ds
type Item = Container<string>.Item;
type View = Buffer<int>.View<float64>;
```

##### Associated Comptime Constants

Class-shaped declarations can also declare associated compile-time values with `comptime const`.
Associated comptime constants are declaration members in static space, not instance fields, and are allowed on any object-like type.

```ds
interface LogStore<Record> {
    comptime const SegmentRows: number = 1024;
    type Segment = Record[this.SegmentRows];
}

class AuditLog implements LogStore<string> {
    comptime const SegmentRows: number = 2048;
}
```

`comptime const` initializers must be statically evaluatable expressions.
In classes and structs, associated comptime constants must have initializers.
In interfaces, associated comptime constants may be abstract (`;`) or defaulted (`= ...`).

Associated comptime constant projections are allowed in static and type-level contexts.
Value-level usage is allowed when the projection is fully resolvable during Analyze and can be folded.
If a value-level projection is not fully resolvable in the current context, we emit an error.

```ds
class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

const ok = SegmentPlan<string>.SegmentBytes + 1;

function unresolved<Row>(): number {
    return SegmentPlan<Row>.SegmentBytes;
}
```

For `unresolved<Row>`, the compiler reports that the associated comptime projection is not resolvable.
Associated comptime constants are compile-time members and thus do not have runtime storage slots.
Inside associated type and associated comptime declarations, `this` refers to the containing owner with outer substitutions applied (as you would expect).

### Enum

Enums mostly follow TypeScript, but remain strictly nominal to avoid accidental implicit coercions.
Explicit casts are required to convert between enums and their backing types.

When member values are omitted, the backing type defaults to the configured integer width.
The backing type is inferred from member values and is either an integer or string type.
- Integer-backed enums allow constant integer expressions using literals, unary +/-, binary arithmetic or bitwise operators, casts, parentheses, and references to earlier enum members.
- Integer-backed enums assign implicit values starting at zero, and explicit values advance the next implicit value by one.
 - String-backed enums require explicit string values for every member.

```ds
// implicitly integer-backed enum
enum Status {
    Active
    Inactive
    Pending
}
```

Because enums are nominal types - effectively, newtypes wrapping their backing type - they can also carry methods and even static members:
```ds
enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,

    static Default = Priority.Low;

    isActive(): boolean {
        this == Status.Active
    }

    label(): string {
        match (this) {
            Active => "Active"
            Inactive => "Inactive"
            Pending => "Pending..."
        }
    }
}
```

### Extension

Extensions add methods to existing nominal types without modifying them.
Unlike TypeScript's prototype extension, Destack extensions are type-safe and scoped.
Extension visibility depends on where the extension is defined relative to the type.

```ds
extension of Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }

    normalized(): Vector2 {
        const m = this.magnitude();
        Vector2 { x: this.x / m, y: this.y / m }
    }
}
```

#### Extension Visibility

| Scenario | Extension Form | Visibility |
|----------|---------------|------------|
| Extension in same file as type | Any | Automatic (wherever type is used) |
| Extension on foreign type, local use | Anonymous | Same file only |
| Extension on foreign type, shared | Named + exported | Where imported |

Extension visibility is thus always explicit per scope:
- When you define a type and extend it in the same file, the methods are part of the type's public API.
- Anonymous extensions on foreign types are private utilities for that file.
- Named extensions can be exported and shared, but must be explicitly imported to use.

#### Extending Newtypes and Primitives

The same mechanism works for newtypes and even precise primitives:

```ds
newtype UserId = int;
extension of UserId {
    isValid(): boolean { this > 0; }
}

// anonymous extension on builtin type: only visible in this file
extension of int32 {
    abs(): int32 { if (this < 0) { -this } else { this }; }
}
```

Since `UserId` is defined in the same file, its extension is visible wherever `UserId` is used.
Since `int32` is a builtin (foreign) type, the anonymous extension is only visible in this file.
(Destack includes a prelude for builtin types that is automatically imported.)

#### Implementing Interfaces

Extensions can implement interfaces, enabling operator overloading:

```ds
interface Add<T, U = T> {
    add(other: T): U;
}
```

```ds
extension of Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}

// now you can use: v1 + v2
```

Multiple overloads for the same operator are supported via multiple interface implementations:

```ds
extension of Vector2 implements Add<Vector2>, Add<float> {
    add(other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }

    add(other: float): Vector2 {
        Vector2 { x: this.x + other, y: this.y + other }
    }
}

// now you can use both: v1 + v2 and v1 + 2.0
```

Operator overloading uses **receiver-based dispatch**: `v1 + v2` becomes `v1.add(v2)`.
The left operand's type determines which implementation "family" is used, then the right operand's type selects the specific overload.
This keeps overload resolution simple and matches TypeScript's method dispatch semantics.

#### Foreign Extensions

Extend types from other modules:

```ds
import { Vector2 } from "somewhere";

// anonymous: only visible in this file (foreign type)
extension of Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

This anonymous extension on `Vector2` is only usable in the file where it's defined.
To share extensions on foreign types, use named extensions and import them.

#### Named Extensions

Named extensions can be exported and must be imported where used:

```ds
// in date-utils.ds
import { Date } from "builtin";

export extension DateUtils of Date implements Add<Date> {
    addDays(days: int): Date { ... }

    add(other: Date): Date { ... }
}
```

```ds
// in app.ds
import { DateUtils } from "./date-utils.ds";

const tomorrow = today.addDays(1);  // works: DateUtils is imported
```

Without importing `DateUtils`, the `addDays` method is not available even if `Date` is in scope.

### Function

Functions work like TypeScript, with the addition that the last expression is implicitly returned.

#### Basic Functions

Functions are dynamically and statically parameterised pieces of reusable logic, just like in JavaScript (and TypeScript).

```ds
function greet(name: string): string {
    `Hello, ${name}!`    // implicit return
}

function add(a: int32, b: int32): int32 {
    a + b                // implicit return
}

function process(data: Data) { // implicit void
    validate(data)
    save(data)
    // no return needed for void
}
```

#### Async Functions

Async functions work like TypeScript:

```ds
async function fetchData(url: string): Promise<Response> {
    const response = await fetch(url);
    response
}
```

Ownership and borrow behavior across `await` and `yield` follows the rules in [References and Values](#references-and-values).

#### Generator Functions

Generator functions use `function*` and `yield`:

```ds
function* fibonacci(): Generator<int32> {
    let (a, b) = (0, 1);
    loop {
        yield a;
        (a, b) = (b, a + b);
    }
}
```
Yielded values must satisfy the `Generator<TYield, TReturn, TNext>` yield type, and yield expressions evaluate to `TNext`.
Return statements inside generator functions must satisfy `TReturn`.

#### Lambda Expressions

Arrow functions work like TypeScript:

```ds
(x) => x * 2
(a: int32, b: int32): int32 => a + b
(items) => {
    for item in items {
        process(item)
    }
}
```

#### Methods

Methods are functions declared inside types (structs, classes, enums, interfaces, extensions).
They receive `this` as an implicit first parameter:

```ds
struct Vector2 {
    x: float32
    y: float32

    // instance method
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }

    // static method
    static zero(): Vector2 {
        Vector2 { x: 0, y: 0 }
    }
}

// calling methods
const v = Vector2 { x: 3, y: 4 };
v.magnitude();        // 5.0
Vector2.zero();       // static call
```

Methods can declare an explicit `this` parameter to constrain the receiver type; the explicit `this` parameter must be first (and does not count toward call arity).
Explicit `this` parameters can use reference types (like `&`) to require mutable receivers.

Member methods have an implicit `this` binding derived from the receiver type.
Non-member functions must declare an explicit `this` parameter to use `this`.

#### Closures

Function values capture lexical bindings from their defining scope.
Captures default to by-value for `const` bindings and by-reference for mutable bindings.
Use `@capture` to override capture mode for specific bindings.
Closure calls implicitly carry their captured environment and do not require explicit arguments.

```ds
const x = 10;
const add = (y: int32) => x + y;
add(5);
```

#### Getters and Setters

Getters and setters work like TypeScript:

```ds
class Person {
    #name: string

    get name(): string {
        this.#name
    }

    set name(value: string) {
        this.#name = value
    }
}
```

#### Constructors

Constructors work like TypeScript:

```ds
class Person {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}
```

#### Rest Parameters and Spread

Rest parameters and spread operators work as in TypeScript:

```ds
function sum(...numbers: int32[]): int32 {
    numbers.reduce((a, b) => a + b, 0)
}

sum(1, 2, 3, 4, 5);

const args = [1, 2, 3];
sum(...args);
```

### Namespace

Namespaces work like in TypeScript:

```ds
namespace math {
    export const PI = 3.14159
    export function sin(x: float64): float64 { /* ... */ }
}
```

### Visibility

Visibility modifiers work like TypeScript:

```ds
class MyClass {
    field: int32; // visible everywhere
    public field: int32; // visible everywhere
    private field: int32; // visible only within the class
    protected field: int32; // visible within the class and subclasses
    #field: int32; // visible only within the class
}
```

## Expressions

**Everything is an expression** in Destack.
Blocks, `if`, `match` all return values:

```ds
const result = if x > 0 { "positive" } else { "negative" };
const label = match (state) {
    Ready => "go"
    Loading => "wait"
};
```

### Bindings

Variable bindings work like TypeScript, with Destack adding tuple destructuring syntax:

```ds
const x = 1;              // immutable
const x: int32 = 1;       // with type
let y = 2;                // mutable

const [a, b] = getTuple();
const { x, y } = getPoint();
const (a, _) = getTuple();  // Destack tuple syntax
```

Unlike JS/TS, bindings can be re-declared in the same scope with a different type (like Rust):

```ds
const x = "hello";        // x is string
const x = x.length;       // x is now int (shadowing)
```

### Blocks

Block expressions group multiple statements and return the last expression's value.
Blocks are enclosed in `{ }` and can optionally have labels:

```ds
const result = {
    const x = compute();
    const y = transform(x);
    x + y                    // last expression is the block's value
};
```

Labeled blocks allow breaking with values:

```ds
const value = outer: {
    for (const i of candidates) {
        if condition(i) {
            break outer i;    // return i from the block
        }
    }
    -1                       // default if no break
};
```

For disambiguation (e.g., after `if` or `match`), use `do`:

```ds
const x = if flag { do { compute() } } else { 0 };
```

### Conditionals

Control flow with `if` works like TypeScript.
Unlike TypeScript, `if` is an expression that returns a value.

```ds
// statement form
if (x > 0) { process(); }

// if/else
if (x > 0) {
    print("positive");
} else if (x < 0) {
    print("negative");
} else {
    print("zero");
}

// as expression - returns a value
const sign = if (x > 0) { 1 } else if (x < 0) { -1 } else { 0 };
const message = if (ready) { "go" } else { "wait" };

// ternary (same as TypeScript)
const sign = x > 0 ? 1 : x < 0 ? -1 : 0;
```

If let is shorthand for matching a value with a pattern in an if condition.
The pattern is matched against the value and selects the then or else branch.
Bindings introduced by the pattern are scoped to the then branch.
If there is no else branch, the if let expression yields void.
If let does not support guards.
Type annotations on if let bindings are treated as `satisfies` constraints on the matched value.

```ds
const result = if let Some(value) = maybe {
    value
} else {
    0
};

if let (x, _) = point {
    print(x)
}

if let x: int32 = value {
    print(x)
}
```

### Match

Destack adds `match` expressions for exhaustive pattern matching.
Like `if`, `match` is an expression that returns a value:

```ds
// match as expression - returns the matched arm's value
const label = match (state) {
    Ready => "go"
    Loading => "wait"
    Error(e) => `failed: ${e}`
};

// match on values
match (value) {
    0 => "zero"
    1 | 2 | 3 => "small"
    n if n < 0 => "negative"
    _ => "other"
}

// match with destructuring
match (point) {
    (0, 0) => "origin"
    (x, 0) => `x-axis at ${x}`
    (0, y) => `y-axis at ${y}`
    (x, y) => `at (${x}, ${y})`
}

// match with guards
match (user) {
    User { age } if age >= 18 => "adult"
    User { age } if age >= 13 => "teen"
    _ => "child"
}
```

Match exhaustiveness is enforced when the compiler can prove the value set is finite:
- enums
- literal unions (`"a" | "b" | 1 | 2`)
- discriminated unions (where every member has the same required literal key with a literal value)

Irrefutable patterns also satisfy exhaustiveness.
This includes `_`, binding patterns, tagged nominal patterns applied to their exact type, and fixed-size sequence patterns applied to fixed-size sequences.

When the compiler cannot prove exhaustiveness (for example when any arm has a guard, or when the type is not a finite set), a `_` fallback arm is required.

TypeScript's `switch` also works unchanged, of course.

```ds
switch (value) {
    case 0:
        return "zero";
    case 1:
        return "one";
    default:
        return "other";
}
```

### Loops

Destack supports all TypeScript loop forms plus `loop`.

#### For-Of and For-In

Iterate over iterables with `for...of` (values) or `for...in` (keys), matching TypeScript exactly:

```ds
for (const item of items) {
    process(item);
}

for (const key in object) {
    console.log(key);
}

// with destructuring
for (const [key, value] of map) {
    console.log(`${key}: ${value}`);
}
```

#### While

Standard while loops work like TypeScript:

```ds
while (condition) {
    process();
}

// do-while
do {
    process();
} while (condition)
```

#### Loop

A `loop` is basically a nicer `while(true)`. That's it.
Infinite loop that can be exited only with `break`:

```ds
loop {
    const input = readInput();
    if (input == "quit") {
        break;
    }
    process(input);
}
```
`loop` expressions can return a value when exited with `break value`, and the result type is the union of break value types.

#### Traditional For

TypeScript's traditional for loop also works:

```ds
for (let i = 0; i < 10; i++) {
    print(i)
}
```

#### Labeled Breaks

All loop forms support labeled breaks and continues:

```ds
outer: for (const i of rows) {
    for (const j of cols) {
        if (condition) {
            break outer      // exits outer loop
        }
        if (other) {
            continue outer   // continues outer loop
        }
    }
}
```

### Patterns

Destack extends TypeScript's destructuring with full pattern matching.
Patterns can appear in `match` arms, `let`/`const` bindings, and function parameters.

#### Wildcard

Matches anything and discards the value:

```ds
_                    // ignore this value
```

#### Binding

Captures a value into a variable:

```ds
x                    // bind to x
var x                // bind to mutable x
```

#### Must

Matches any non-nullish, Try-unwrapped value and binds it:

```ds
x!                   // bind only if value is not null or undefined
```

#### Literal

Matches exact values:

```ds
42                   // match integer
"hello"              // match string
true                 // match boolean
```

#### Tuple

Matches tuple structure:

```ds
(a, b)               // two-element tuple
(x, _, z)            // ignore middle element
(first, ...rest)     // rest pattern
```

#### Object

Matches object/struct properties:

```ds
{ x, y }             // shorthand
{ x: a, y: b }       // rename bindings
{ x, ...rest }       // rest pattern
Point { x: 0, y }    // tagged with literal field
```

Tagged object patterns accept any object-like type expression, including type aliases and interfaces.
The tag must resolve to an object type, or the pattern is a type error.

#### Array/Slice

Matches array elements:

```ds
[a, b, c]            // exact three elements
[first, ...rest]     // first + rest
[first, ..., last]   // first and last
[]                   // empty array
```

#### Variant

Matches enum/union variants:

```ds
Some(x)              // unwrap Some
Ok(value)            // unwrap Ok
Result.Err(e)        // qualified path
```

#### Union

Matches any of several patterns:

```ds
1 | 2 | 3            // match 1, 2, or 3
"a" | "b"            // match either string
```

#### Guards

Add conditions to patterns (only in `match`):

```ds
n if n > 0           // positive numbers only
x if x.isValid()     // with method call
```

### Try, Results and Errors (and Exceptions)

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while exceptions exist for compatibility, interop, and migration.
The builtin `Error` interface is the conventional error shape, but any type can be used as `E`.
`Result<T, E>` with `?` and `??` remains the preferred everyday style.

#### Try protocol

The `Try<T, E>` interface defines the protocol for `?` and `??`.
`Try` is nominal, while its branch shape is structural.
`Try.branch()` must return a `TryBranch<T, E>` compatible shape:

```ds
type TryBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };
```

#### Try and the ? Operator

The `?` operator immediately propagates `Try` error variants to the caller:

```ds
function readConfig(path: string): Result<Config, Error> {
    const text = readFile(path)?;    // returns early if Err
    const json = parseJson(text)?;   // returns early if Err
    return Result.ok(Config.from(json));
}
```

When `?` is applied to a `Try<T, E>`:
- If the branch is ok, extracts and returns the success value.
- If the branch is err, returns early with `Try.fromError(error)` from the enclosing function.
- The receiver must be a non-nullish `Try` type.
- The `?` operator unwraps at most one `Try` layer.

And, of course, The enclosing function must return a compatible `Try` type.
The error type `E` is unconstrained, but `Error` is the conventional shape.

#### Try and the ?? Operator

The `??` operator extracts the success value or uses a default:

```ds
const config = loadConfig() ?? defaultConfig;
const port = parsePort(input) ?? 8080;
```

When `??` is applied to a `Try<T, E>`:
- If the branch is ok, extracts and returns the success value.
- If the branch is err, returns the right-hand default value.
- If `T` is nullish, returns the right-hand default value.

Like `?`, `??` performs at most one `Try` unwrap.
When the left-hand side is a union of `Try` and non-`Try` values, the result unions the unwrapped success types, non-`Try` members, and the fallback.
Nullish values are removed from both the union and the `Try` success type.

```ds
declare const anonymousUser: User;
const value: Result<User, IOError> | null = loadUser();
const user: User = value ?? anonymousUser;
```

#### try/catch with Result and exceptions

The `try`/`catch` syntax handles exceptions and explicit `Try` propagation:

```ds
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e: IOError) {
    log("Failed:", e)
} finally {
    cleanup()
}
```

ALl types implementing `Try` support `try`/`catch` propagation, including the standard lirbary `Result`:
 - Note that `try` does not implicitly unwrap `Result` values.
 - Use `?` or `??` inside the block to propagate `Try` errors into the catch.
 - When a `?` is inside a `try` with a catch, `Try.fromError` is not required.

Thrown exceptions propagate into the catch in the usual way when exceptions are enabled.
As usual, a try expression must include a catch or finally block.

```ds
try {
    riskyOperationA()?;
} catch match (e) {
    NumericError(x) => Error(`bad number: ${x}`))
    FormatError => Error(`bad format ${e}`))
    _ => Error(`unknown error: ${e}`))
}
```

#### throw and native exceptions

`throw` supports exception-style control flow for compatibility with JS/TS and with exception-oriented ecosystems like Java and C#.
Destack still prefers `Result` for ordinary recoverable errors, especially in performance-critical code.

```ds
function unwrap<T>(r: Result<T, Error>): T {
    match (r) {
        Ok { value } => value
        Err { error } => throw error   // panic: caller made a mistake
    }
}
```

### Using
`using` declares a resource that will be disposed when the current **lexical scope** exits.
Destack mirrors JS/TS semantics and syntax, following the TC39 Explicit Resource Management proposal.

```ds
using file = openFile(path);
await using conn = openConnection();
```

#### Declarations
`using` and `await using` appear anywhere a lexical declaration is allowed.
They always have an initializer and are immutable like `const`.
`await using` is only allowed where `await` is legal.
`export using` is valid and behaves like `export const`.

```ds
using a = openA(), b = openB();
export using cache = openCache();
```

#### Disposal
Resources are disposed when the scope exits for any reason (fallthrough, `return`, `break`, `continue`, `throw`, `?`), and never later than scope exit.
Disposal is **LIFO**, so later declarations are disposed first.
Disposal continues even if earlier disposals throw.
`SuppressedError` is used when both a body error and a disposal error exist, matching JS.
`await using` prefers `[Symbol.asyncDispose]()` if present, otherwise `[Symbol.dispose]()`.
The result is awaited.
`null` and `undefined` are no-ops.

```ds
using first = getFirst();
using second = getSecond();
```

#### Type requirements
`using` requires `Disposable | null | undefined`.
`await using` requires `AsyncDisposable | Disposable | null | undefined`.
`Disposable` is structural and requires a `[Symbol.dispose](): void` method.
`AsyncDisposable` is structural and requires a `[Symbol.asyncDispose](): PromiseLike<void> | void` method.

```ds
declare const resource: Disposable;
using value = resource;
```

#### Loops
In `for`/`for..of`/`for..in` initializers, a `using` resource is per-iteration and disposed at the end of each iteration.
The disposal runs on `continue` and `break`, just like normal scope exit.

```ds
for (using line of readLines(path)) {
    process(line);
}
```

#### Modules
Top-level `using` in modules disposes when module evaluation completes.
This includes completion after top-level `await`.

```ds
using log = openLog();
await run();
```

## Operators

Destack operators match TypeScript with additional precision, e.g., for wrapping or saturating integer arithmetic.
Most operators can be overloaded via extensions implementing the corresponding interface.
Operators with an interface in the table below desugar to method calls on the left operand (receiver-based dispatch).
See [Dispatch](#dispatch) for overload ordering and union based dynamic resolution.

The receiver type determines which implementation "family" to look at, and the right operand type selects the specific overload within that family.
For example, `Vector2` can implement both `Add<Vector2>` and `Add<float>` for vector addition and scalar addition respectively.

### Operator Interfaces

Operator interfaces are declared as **nominal interfaces** using `newtype interface`:

```ds
newtype interface Add<T, R = this> {
    add(other: T): R
}
```

Because they are nominal, operators only overload when a type explicitly `implements` that operator interface.
(This prevents accidental conformance from types that happen to have a structurally-compatible method. Symbol-branding with something like `Operator.add` was considered, but rejected, since nominal interfaces are a stronger contract and more readily apparent statically.)

```ds
// Foo has an `add` method but doesn't implement Add<T>
struct Foo {
    add(other: Foo): Foo { ... }
}

const a = Foo { };
const b = Foo { };
a + b;        // error: Foo does not implement Add
a.add(b);     // ok: direct method call works

// Foo explicitly implements Add<T>
extension of Foo implements Add<Foo> {
    add(other: Foo): Foo { ... }
}

a + b;        // ok: Foo implements Add<Foo>
```

This rule applies to all operator interfaces (see [Nominal Interfaces](#nominal-interfaces)).

### Arithmetic

Standard arithmetic operators, all overloadable:

| Operator | Description | Interface | Assignment |
|----------|-------------|-----------|------------|
| `+` | Add | `Add<T, U = T>` | `+=` |
| `-` | Subtract | `Subtract<T, U = T>` | `-=` |
| `*` | Multiply | `Multiply<T, U = T>` | `*=` |
| `/` | Divide | `Divide<T, U = T>` | `/=` |
| `%` | Remainder | `Remainder<T, U = T>` | `%=` |
| `**` | Power | `Power<T, U = T>` | — |
| `-a` | Negate (unary) | `Negate<U = this>` | — |
| `+a` | Plus (unary) | `Plus<U = this>` | — |

Destack adds explicit overflow control for integer types:

| Standard | Wrapping | Saturating | Description |
|----------|----------|------------|-------------|
| `+` | `+%` | `+\|` | Add |
| `-` | `-%` | `-\|` | Subtract |
| `*` | `*%` | `*\|` | Multiply |

- **Standard**: default behavior
- **Wrapping** (`%`): overflow wraps around (like C unsigned)
- **Saturating** (`|`): overflow clamps to min/max

### Overflow Control

The standard operators follow the target safety policy (`overflowChecks` or `safetyPreset`).
When overflow checks are enabled, `+`, `-`, and `*` trap on overflow.
When overflow checks are disabled, those operators wrap in two's complement.
Division and remainder trap on division by zero and signed `min_value / -1` in safe modes.
Unchecked builds lower `/` and `%` to unchecked operations.

Runtime checks are configured per target:
- `safetyPreset`: sets default policies for all runtime checks
- `overflowChecks`: controls overflow checking for `+`, `-`, `*`
- `boundsChecks`: controls array and slice bounds checks
- `nullChecks`: controls null checks on reference operations
- `divisionChecks`: controls divide and remainder checks
- `shiftChecks`: controls shift range checks
- `checkFailure`: controls how check failures are handled (`trap`, `panic`, `abort`)
Explicit per-check settings override the preset.

```ds
const a: uint8 = 250;
const b: uint8 = 10;

a + b;          // default overflow behavior
a +% b;         // wrapping: 250 + 10 = 4 (wraps around 256)
a +| b;         // saturating: 250 + 10 = 255 (clamped to max)
```

### Comparison

Comparison operators for equality and ordering:

| Operator | Description | Interface |
|----------|-------------|-----------|
| `==` | Equal | `Equal<T>` |
| `!=` | Not equal | `Equal<T>` |
| `<` | Less than | `Compare<T>` |
| `<=` | Less than or equal | `Compare<T>` |
| `>` | Greater than | `Compare<T>` |
| `>=` | Greater than or equal | `Compare<T>` |
| `===` | Strict equal | — |
| `!==` | Strict not equal | — |

Strict equality (`===`, `!==`) is not overloadable to preserve JavaScript's identity semantics.

### Logical

Logical operators with short-circuit evaluation (not overloadable):

| Operator | Description | Interface |
|----------|-------------|-----------|
| `&&` | Logical and | — |
| `\|\|` | Logical or | — |
| `!` | Logical not | — |
| `??` | Nullish coalescing | — |

### Elementwise

Elementwise operators for bitwise operations on integers, overloadable for other types (sets, boolean arrays, SIMD vectors):

| Operator | Description | Interface | Assignment |
|----------|-------------|-----------|------------|
| `&` | Elementwise and | `And<T, U = T>` | `&=` |
| `\|` | Elementwise or | `Or<T, U = T>` | `\|=` |
| `^` | Elementwise xor | `Xor<T, U = T>` | `^=` |
| `~` | Elementwise not | `Not<U = this>` | — |
| `<<` | Shift left | `ShiftLeft<T, U = T>` | `<<=` |
| `>>` | Shift right | `ShiftRight<T, U = T>` | `>>=` |
| `>>>` | Unsigned shift right | `ShiftRightUnsigned<T, U = T>` | `>>>=` |
| `<<\|` | Saturating shift left | `ShiftLeftSaturating<T, U = T>` | `<<\|=` |

### Indexing

Index operators for subscript access and assignment (like Python's `__getitem__`/`__setitem__` or Rust's `Index`/`IndexMut`):

| Operator | Description | Interface |
|----------|-------------|-----------|
| `a[i]` | Index access | `Index<I, O>` |
| `a[i] = v` | Index assignment | `IndexSet<I, V>` |

Array and tuple indexing is bounds checked and returns the element type.
Out of bounds accesses trigger the configured bounds check failure.
`noUncheckedIndexedAccess` only affects index signatures and other dynamic indexers.
String index signatures accept numeric index expressions because numeric keys coerce to strings.
Number index signatures accept numeric indices and numeric string literals that are canonical JS numeric names.
Symbol index signatures accept symbol keys and `keyof` yields `symbol`.

### Type Operators

Type-level operators (not overloadable):

| Operator | Description | Interface |
|----------|-------------|-----------|
| `as` | Type cast, plus `as comptime` disambiguation in type indexes | — |
| `in` | Key membership check | — |
| `is` | Type guard | — |
| `instanceof` | Class identity check | — |
| `satisfies` | Type satisfaction | — |
| `typeof` | Get value type | — |
| `keyof` | Get keys | — |
| `extends` | Subtype check | — |
| `implements` | Interface check | — |

The `is` operator uses `T.is` when runtime checks are required.
The `instanceof` operator is only defined for class identity checks.
The `in` operator returns a boolean literal when the key is assignable to `keyof` of the right type.

For unions, `keyof (A | B)` is the intersection of keys present on every union member.
For intersections, `keyof (A & B)` is the union of keys from all intersected types.
The `in` operator follows the same `keyof` rules for unions and intersections.
The `extends` and `implements` operators return boolean literals (`true` or `false`) when assignability is decidable and `boolean` otherwise.

### Mapped Types

Mapped types construct new object types by iterating over keys.
Destack supports all the TS built-in mapped types, defined in the exact same way:

```ds
type Flags<T> = { [K in keyof T]: boolean };
type Optional<T> = { [K in keyof T]?: T[K] };
type Required<T> = { [K in keyof T]-?: T[K] };
type Frozen<T> = { readonly [K in keyof T]: T[K] };
type Mutable<T> = { -readonly [K in keyof T]: T[K] };
```

Key remapping is also supported with `as`:

```ds
type Renamed<T> = { [K in keyof T as "value"]: T[K] };
```

### Other Operators

Additional operators (not overloadable):

| Operator | Description | Interface |
|----------|-------------|-----------|
| `?.` | Optional member access | — |
| `?` | Optional unwrap | — |
| `!` | Non-null assertion | — |
| `...` | Spread / rest | — |
| `++` / `--` | Increment / decrement | — |

### Builtin Overloading

Builtin primitive types (`number`, `int32`, `string`, etc.) have implicit operator implementations.
Destack also provides (optional) operator overloading for standard library types via `.d.ds` declarations:

| Type | Operators | Notes |
|------|-----------|-------|
| `Date` | `<`, `>`, `<=`, `>=`, `==` | Date comparison |
| `Set<T>` | `&` (intersection), `\|` (union), `-` (difference) | Set algebra |
| `Array<T>` | `+` (concatenation) | Replaces string coercion |
| `TypedArray` | `+`, `-`, `*`, `/` (elementwise) | SIMD-style operations |
| `Map<K,V>` | `\|` (merge) | Map merging |

These overloads transpile to explicit method calls in the generated TypeScript.

## Dispatch

See [Declarations](#declarations) for function and method declaration forms.
See [Operators](#operators) for operator specific dispatch behavior.

### Function Overloading

Destack supports function overloading without the clumsy type-only declarations:

```ds
function parse(input: string): int32 {
    parseInt(input)
}

function parse(input: int32): int32 {
    input
}

// both are callable
parse("42")   // calls first
parse(42)     // calls second
```

### Overload Resolution

Overloads are resolved using **declaration order**: the first matching overload wins (like in TypeScript).
Applicability includes static parameter inference and validation, including `comptime` value parameters.
Overload order is defined at the declaring module and is forwarded unchanged across exports, reexports, and namespace imports.

The process has two phases:

- Applicability: a candidate is applicable only when its static and dynamic parameters can be satisfied at the call site
- Selection: among applicable candidates, the first candidate in declaration order wins

```ds
// good: specific overloads before general ones
function format(x: "json"): JsonFormatter;
function format(x: "xml"): XmlFormatter;
function format(x: string): Formatter;

format("json")    // calls first overload
format("xml")     // calls second overload
format("csv")     // calls third overload
```

```ds
// bad: general overload shadows specific ones
function format(x: string): Formatter;
function format(x: "json"): JsonFormatter;  // warning: shadowed by first overload

format("json")    // calls first overload (not second!)
```

The compiler warns when an overload is shadowed by an "earlier" declaration that always matches first.
For union argument types, the argument must be assignable to a single overload:

```ds
function handle(x: string): string;
function handle(x: number): number;

const y: string | number = getValue();
handle(y);  // error: union argument not assignable to any overload
```

```ds
function handle(x: string | number): string | number;

const y: string | number = getValue();
handle(y);  // ok
```

#### Dynamic Resolution on Unions

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant.
Dynamic resolution applies when **all** union variants expose the member and the resolved targets differ across variants.
- If any union variant lacks the member, the access is an error.
- If all variants resolve to the same target symbol and resolved signature, the compiler may treat the resolution as static.
- If the target symbols or resolved signatures differ, the resolution is dynamic and dispatches on the receiver type.
- Extension methods participate in member resolution for each variant.

For method calls on unions, the call must be valid for every candidate signature:
- Arguments must satisfy each candidate signature.
- The return type is the union of per-candidate return types (after substitutions).

Dynamic resolution is implemented by reifying the call into `if (receiver is Type)` branches with statically resolved calls in each branch.

## Annotations

Destack has three kinds of annotations: comments, documentation, and decorators.
All annotations are preserved in the AST and available to tooling.
(The Destack AST is a superset of the TypeScript AST that also contains whitespace and concrete info like a traditional CST).
See [Reflection](#reflection) for runtime metadata access.

### Decorators

Decorators generalize TypeScript decorator semantics to enable decorators both as metadata and as transforms on most language constructs: declarations, statements, members, parameters, match arms, and more (not just classes and class members).

```ds
@memoize                           // decorator: memoize(target)
@route("/api/users")               // factory: route("/api/users")(target)
@service.middleware                // member access: service.middleware(target)
```

The decorator LHS-expression can be any expression (path, member access, call):
- `@foo` → calls `foo(target)`
- `@foo(args)` → calls `foo(args)(target)` (factory pattern)
- `@obj.method` → calls `obj.method(target)`

Decorators can be applied to most language constructs:

```ds
// on declarations
@deprecated("use newAPI")
function oldAPI() { }

// on statements
@unroll
for (let i = 0; i < 4; i++) { }

// on struct/class members
struct Config {
    @env("DEBUG")
    debug: boolean,
}

// on function parameters
function greet(@validate name: string) { }

// on reference types
function kernel(data: @addrspace("shared") &Point) { }

// on match arms
match (event) {
    @likely
    Click(pos) => handleClick(pos),
    @cold
    Error(e) => logError(e),
}
```

#### Decorator Resolution

Decorator behavior depends on what the decorator resolves to:

| Resolves to | Behavior |
|-------------|----------|
| **Function** | Transforms target at runtime (standard decorator semantics) |
| **Newtype** | Compile-time metadata only, stripped in output |

Newtype-based decorators enable compiler hints without runtime overhead:

```ds
newtype unroll = void;
newtype inline = void;
newtype deprecated = string;
newtype addrspace = (string,) | (int,);

@unroll                    // hint to compiler, stripped in JS output
for (let i = 0; i < 4; i++) { }

@deprecated("use newAPI")  // compile-time warning, stripped in JS output
function oldAPI() { }

@addrspace("shared")       // type metadata, lowered to explicit address space
function kernel(data: @addrspace("shared") &Point) { }
```

### Comments

Standard JS/TS comments:

```ds
// line comment
/* block comment */
```

### Documentation

Documentation comments are attached to the following declaration and used for generated docs:

```ds
/// Line documentation comment.
/// Can span multiple lines.

/**
 * Block documentation comment.
 * Supports markdown formatting.
 */
```

## Trees

Tree literals generalize TSX/JSX syntax for any tree-shaped data beyond UIs with a superset of JSX/TSX syntax.

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

Like in TSX, Destack's tree literals come in two main forms:
- **Value tags** via `TreeTag`: uppercase qualified tags like `<Button />` and `<UI.Button />` resolve in the normal value namespace.
- **Intrinsic tags** via `TreeTagBuilder`: lowercase unqualified tags like `<div />` and `<span />` resolve through the active `TreeTagBuilder`.

Value tag resolution follows normal symbol resolution rules, including import and member lookup semantics, just like in regular TS/TSX.
Intrinsic tags resolve through the active `TreeTagBuilder` as compile-time string names.
Namespaced XML tags like `<svg:path />` keep their lexical name and route as intrinsic tag names (`"svg:path"`).

### Value tags (TreeTag)

Most "custom" tags are value tags, i.e., the uppercase `<Component />`s that are defined as actual types somewhere.
Value tree tag construction is defined by the `TreeTag` interface.
Compatible types automatically get a `TreeTag` implementation when possible, and all (nominal) types can implement `TreeTag` manually to customize tree construction behavior.

```ds
/// A tree tag.
newtype interface TreeTag {
    /// The associated Props type.
    type Props;
    /// The associated Node type.
    type Node;

    /// Create a Node from props and children.
    static fromTree(props: this.Props, children: readonly this.Node[]): this.Node;
}
```

### Intrinsic tags (TreeTagBuilder)

Like JSX/TSX, Destack also supports "intrinsic" lowercase tags that resolve to a generic tag instead of a specific type.
Lowercase tags and fragment construction are defined by the `TreeTagBuilder` interface.

```ds
/// A tree tag builder for intrinsic string tags (like `<div />` or `<svg:path />`)
newtype interface TreeTagBuilder {
    /// The associated Node type for all routed intrinsic tags.
    type Node;
    /// The tag with the given name.
    type Tag<comptime Name: string>: TreeTag;
    /// The fragment type (for `<> ... </>`).
    type Fragment: TreeTag;
}
```

The active tree tag builder can be configured at workspace, project, and module level.
Destack also recognizes `jsx`, `jsxFactory`, `jsxFragmentFactory`, and `jsxImportSource` options for TSX ecosystem compatibility.

## Modules

Module syntax matches JS/TS exactly.
See [Targets](#targets) for target specific module behavior.
See [Compatibility](#compatibility) for supported file type matrix details.

### Imports

Imports work exactly like JS/TS.

#### Side-Effect Import

Import a module for its side effects only:

```ds
import "module";
```

#### Named Imports

Import specific exports by name:

```ds
import { foo, bar } from "module";
```

#### Aliased Import

Rename an import locally:

```ds
import { foo as f } from "module";
```

#### Namespace Import

Import all exports as a namespace object:

```ds
import * as mod from "module";

mod.foo();
mod.bar;
```

#### Default Import

Import the default export:

```ds
import Default from "module";
```

#### Combined Import

Import default and named together:

```ds
import Default, { foo, bar } from "module";
```

#### Type-Only Import

Import only types (erased at runtime):

```ds
import type { MyType } from "module";
import { type MyType, myValue } from "module";
```

#### TypeScript Triple-Slash Directives

TypeScript source files support leading triple-slash reference directives.
`reference path`, `reference types`, and `reference lib` are treated as type-only dependency edges.

```ts
/// <reference path="./global.d.ts" />
/// <reference types="node" />
/// <reference lib="dom" />
```

### Exports

Exports work exactly like JS/TS.

#### Inline Export

Export declarations directly:

```ds
export const value = 42;
export function foo() { }
export struct Point { x: float32, y: float32 }
```

#### Named Exports

Export previously declared items:

```ds
const a = 1;
const b = 2;

export { a, b };
```

#### Aliased Export

Export with a different name:

```ds
export { internal as public };
export { foo as default };     // as default export
```

#### Default Export

Export a single default value:

```ds
export default function handler() { }
export default class MyClass { }
export default expression;
```

#### Re-Export

Forward exports from other modules:

```ds
export { foo, bar } from "module";    // specific items
export * from "module";               // all exports
export * as ns from "module";         // as namespace
```

#### Type-Only Export

Export only types (erased at runtime):

```ds
export type { MyType };
export { type MyType, myValue };
```

#### Export Inference in Cyclic Module Graphs

Export inference is the cross-module cycle breaker for Analyze.
The compiler evaluates export dependencies as strongly connected components (SCCs).
An SCC is accepted only when every exported value binding in that SCC converges to a concrete type.
Declared annotations seed the solve, but acceptance is based on solvedness, not an anchor count heuristic.
If any exported value in the SCC remains unsolved after surface convergence, that binding is rejected and requires an explicit annotation.

Unsolved two-module cycle:

```ts
// a.ts
import { y } from "./b";
export const x = y;

// b.ts
import { x } from "./a";
export const y = x;
```

This SCC is rejected because neither `x` nor `y` is solved.

Solvable three-module SCC with one anchor:

```ts
// a.ts
import { y } from "./b";
export const x: number = y;

// b.ts
import { z } from "./c";
export const y = z;

// c.ts
import { x } from "./a";
export const z = x;
```

This SCC is accepted because the one annotation on `x` determines `z` and then `y`.

Solvable namespace cycle with one anchor:

```ts
// a.ts
import * as b from "./b";
export const x: number = b.y;

// b.ts
import * as a from "./a";
export const y = a.x;
```

This SCC is accepted because both exports converge to `number`.

### Data Imports

Destack supports importing non-code files with automatic type inference.

#### JSON, TOML, YAML

Data files are parsed and typed structurally:

```ds
import config from "./config.json";

config.name     // string
config.port     // number
config.debug    // boolean
```

Type inference rules:
| JSON Value | Inferred Type |
|------------|---------------|
| `null` | `null` |
| `true`/`false` | `boolean` |
| Numbers | `number` |
| Strings | `string` |
| Arrays | `T[]` or `(T \| U \| V)[]` for mixed elements |
| Objects | `{ key: Type, ... }` with readonly fields |

Empty arrays infer element type `unknown`.

#### Text Files

Text files (`.md`, `.txt`, `.css`, `.html`, `.svg`, etc.) import as `string`:

```ds
import content from "./README.md";
// content: string
```

#### Binary Files

Binary files (images, fonts, wasm, etc.) import as `uint8[]`:

```ds
import data from "./image.png";
// data: uint8[]
```

#### Import Attributes

Override the default loader using import attributes:

```ds
import data from "./file.toml" with { type: "json" };   // parse as JSON
import raw from "./data.json" with { type: "text" };    // import as string
import bytes from "./file.txt" with { type: "binary" }; // import as uint8[]
```

Valid loader types:
| Type | Description |
|------|-------------|
| `json` | Parse as JSON, infer structural type |
| `toml` | Parse as TOML, infer structural type |
| `yaml` | Parse as YAML, infer structural type |
| `text` | Import as `string` |
| `binary` | Import as `uint8[]` |
| `base64` | Encode binary as base64 `string` |

The same file with different loaders produces different modules:

```ds
import a from "./data.json";                           // parsed JSON
import b from "./data.json" with { type: "text" };    // raw string
// a and b are different modules
```

## Ownership

See [Types](#types) for base type forms and generic constraints.
See [Expressions](#expressions) for control flow forms that interact with lifetimes.
See [Targets](#targets) for strict target defaults that affect ownership checking.

### References and Values

Like in TypeScript, a plain `T` follows the default semantics of its type: objects are GC-managed references, primitives are copied values.
Ownership is orthogonal to identity semantics, which are determined by the base type.
Ownership modifiers change storage and lifetime but do not change whether a type has identity.
This keeps `struct` value semantics and `class` identity semantics consistent across all ownership modes.
Destack also supports explicit ownership control:

```ds
T            // type default (value or managed reference)
&T           // borrow (mutable, exclusive reference)
&readonly T  // borrow (read only reference)
^T           // owning handle (move-only)
^readonly T  // owning handle (move-only, readonly)
```

### Raw pointers

Raw pointers are also supported:
```ds
*T           // raw pointer (mutable, unsafe)
*readonly T  // raw pointer (readonly, unsafe)
```

- `*T` and `*readonly T` are unsafe pointers with no borrow tracking.
- Raw pointers may be null or dangling and allow pointer arithmetic.
- Converting between borrowed references and raw pointers is always explicit.
- Raw pointers do not imply ownership or drop behavior.
- Raw pointers only support equality and inequality comparisons.
- `&const T` and `*const T` are accepted but redundant and format as `&readonly T` and `*readonly T`.

| Modifier | Meaning | After `foo(x)` | Who cleans up? |
|----------|---------|----------------|----------------|
| `T` | Type default (value or managed reference) | `x` still valid | Type default |
| `&readonly T` | Borrow (read) | `x` still valid | Original owner |
| `&T` | Borrow (mutate) | `x` still valid, maybe changed | Original owner |
| `^T` | Owning handle (move-only) | `x` **invalid** | New owner (raw allocation) |
| `^readonly T` | Owned reference (read only) | `x` **invalid** | New owner (raw allocation) |

Managed reference types are collected by the GC.
Value types only drop when owned or used with `using`.
For value types, `T` is an inline value with copy or move semantics, while `^T` is an owning handle that allocates and transfers ownership.
Use `^T` when you need explicit ownership transfer, deterministic drop, or to avoid copying large value types.
Use `^readonly T` when ownership transfer is required but mutation must be forbidden.
Use plain managed `T` when you want ordinary object identity without forcing physical address stability.

**Implicit managed defaults:**
`noImplicitManaged` requires explicit ownership operators anywhere a type or value would otherwise use managed defaults.
Managed defaults include classes, interfaces, structural object types, arrays, functions, and string-like literals.
Explicit ownership operators (`^T`, `&T`, `*T`) satisfy the requirement for both type annotations and inferred values.
`noManaged` forbids GC-managed defaults and allocations.
Explicit ownership operators remain valid and use raw allocation.

**Use after move:**

```ds
const node = AstNode { ... };
consume(^node);    // ownership transferred
print(node.value); // error: use after ownership transfer
```

`^T` is the owning handle type.
`^T` follows affine semantics: an owned value can be moved at most once and cleaned up at most once.
Passing a `^T` by value transfers ownership to the callee.
Use `^expr` to convert a value `T` into an owning handle `^T`.
If you already have `^T`, pass it directly instead of writing `^expr`.
`Copy` and `Clone` do not weaken `^T` affine semantics.
They apply to plain values and explicit duplication, not to implicit owner taking fallback.
If a function requires `^T`, the argument is moved.
The compiler does not silently copy a `Copy` value just to satisfy an owning parameter.

Using a value after ownership transfer is an error (suppressible to warning).

**Cleanup as soon as possible:**

`^T` values are dropped at their last proven use (non lexical).
The compiler inserts a drop as soon as it can prove the value is no longer needed,
even if the lexical scope continues.

```ds
function process() {
    const data = ^LargeData { ... };  // we own this
    doWork(&readonly data);            // borrow it
    log("done");                       // data can be dropped before this line
}
```

### Borrow checking

Destack's borrow checking is provenance based, i.e., based on the origin roots of a borrowed value (similar to Mojo).
Each borrow has one or more origin roots that identify the owners or storage locations it depends on.

**Origin roots:**

- A borrow taken from a parameter has that parameter as its origin root.
- A borrow taken from a local, stack allocation, global, or owned value has that storage as its origin root.
- A borrow returned from a call uses the roots described by the callee's lifetime contract.
- A borrow of static or global only data uses the `Static` origin.

**Derived borrows:**

- Reborrowing a borrow preserves the same origin roots.
- Borrowing a field, element, or view of a value derives a borrow with the same roots as the base value.
- Casting between borrow forms preserves provenance when the conversion is safe and non-owning.

**Merged borrows:**

- When control flow merges borrows from different origins, the resulting borrow carries the union of all possible roots.
- Function lifetime summaries may therefore mention multiple parameters.
- The compiler may reject a borrow only when it cannot represent the required provenance conservatively.

**Invalidation:**

- Moving an owned value invalidates the previous binding and every borrow rooted in it.
- Dropping or freeing a value invalidates every borrow rooted in that value.
- Mutating through an exclusive path invalidates conflicting readonly borrows according to the borrow rules.
- Storing through raw pointers or unknown aliases may conservatively invalidate precise provenance facts.

**Lifetime summaries:**

- A function lifetime annotation describes the provenance roots that a returned borrow may depend on.
- It does not encode the entire internal borrow graph or every derived reborrow step.
- The inferred default follows normal receiver and parameter based borrowing conventions.

**Suspension points (`await` and `yield`):**

A suspension point is any `await` or `yield` boundary.
Owned values (`^T`) may cross suspension points by moving into the coroutine frame.
Borrowed references (`&T`, `&readonly T`) must not remain live across suspension points in strict mode.
In lenient mode, the same rule violations are warnings.
If an owned value is dead before suspension, cleanup is inserted before the suspend edge.
This rule is especially important for borrows of managed storage because a borrow does not imply a stable raw address across suspension or relocation points.

```ds
async function bad(parent: ^Parent) {
    const child: &readonly Child = &readonly parent.child;
    await tick();
    use(child); // error in strict mode: borrow held across await
}
```

References can also target explicit address spaces for native and accelerator memory.
The default is `generic`, which maps to the target's normal memory.
Non generic address spaces apply to borrowed and raw references.
Managed references carry logical object identity instead of an address space qualified native pointer.
Pinned managed state is live runtime state and is not itself part of durable image or snapshot semantics.

**Nested ownership:**

Ownership is at the usage site, not the definition site.
Structs can contain `^T` fields regardless of how the struct itself is stored:

```ds
struct Container {
    data: ^Data; // Container.data is owned by Container
}

const value: Container = ...;          // value semantics container
const owned: ^Container = ...;         // owned container
```

Moving the owner of a container also moves any owned fields inside that container.

When the container is `^Container`, cleanup is deterministic and fields are cleaned up in reverse declaration order before the container itself.
When the container is boxed into managed storage, cleanup timing is nondeterministic and follows GC behavior.
In strict mode, the compiler warns for `^T` fields inside managed container shapes.

### Ownership Conversions

Ownership conversions are explicit, except for borrows inserted at reference boundaries.
Implicit ownership conversions only create borrows and never transfer ownership.
Struct boxing at reference boundaries is a value to reference conversion, not an ownership conversion.

Implicit conversions:
- `T` → `&T` or `&readonly T` when a reference is required and the value is addressable
- `^T` → `&T` or `&readonly T` when a reference is required
- `&T` → `&readonly T` to reborrow as shared

Explicit conversions:
- `T` ↔ `^T` require explicit ownership operators or helper calls (use `^expr` for `T` → `^T`)
- `&T` → `T` requires `Copy` or an explicit clone
- `&T` → `^T` requires an explicit clone and ownership transfer
- `*T` conversions require explicit unsafe operations

Copy and clone conversions:
- Passing plain `T` may copy when the type implements `Copy`.
- Explicit duplication may use `Clone` when the type exposes that protocol.
- Passing `^T` never performs an implicit copy fallback, even when `T: Copy`.
- To produce a second owner, first copy or clone the underlying value, then convert or move that new value explicitly.

Example (implicit borrow):

```ds
function read(item: &readonly Item): int { item.size() }

const value = Item { size: 10 };
read(value);
```

Example (explicit ownership conversion):

```ds
function consume(value: ^Item) { ... }

const value = Item { size: 10 };
consume(^value);
```

If you already have `^Item`, pass it directly without `^`.

Example (scope-end drop):

```ds
using file: ^File = File.open(path);
file.write("hello");
```

**Returning references:**

Functions can return `&readonly T`.
Borrowed returns use lifetime inference and `@lifetime` annotations to track which inputs they borrow from.
In strict mode, returning a borrow that may outlive its origin is an error.
(In lenient mode, the same situation produces a warning).

```ds
function get(c: &readonly Container): &readonly Item { &readonly c.item }  // ok

function bad(): &readonly Point {
    const p = Point { x: 1, y: 2 }
    &readonly p  // WARNING: returning reference to local
}
```

By default, `&T` and `&readonly T` are compiler hints.
Violations of aliasing or suspension rules produce warnings instead of errors.

### Lifetime Annotations

When a function returns `&readonly T` or a type containing borrowed references, the compiler tracks which input parameters the return value borrows from.
This is usually inferred automatically:

| Input Parameters | Inference |
|------------------|-----------|
| Single `&readonly T` parameter | Return borrows from it |
| `&readonly self` or `&readonly this` receiver | Return borrows from receiver |
| Multiple `&readonly T` parameters | Return borrows from all (conservative) |

When conservative inference is too restrictive, use `@lifetime` to specify exactly
which parameters the return borrows from:

```ds
@lifetime("param")          // borrows from single parameter
@lifetime("a", "b")           // may borrow from a or b
@lifetime("static")       // borrows from static/global data only (static is a reserved name anyway)
```

**Examples:**

```ds
// only borrows from 'a', caller knows 'b' can be a local
function first(a: &string, b: &string): @lifetime("a") &string {
    return a;
}

// may borrow from either parameter
function pick(a: &string, b: &string): @lifetime("a", "b") &string {
    if (condition) { return a; }
    return b;
}

// static lifetime: string literal never dies
function constant(): @lifetime("static") &string {
    return &"hello";
}

// struct containing borrowed reference
struct Tokenizer { source: &string, pos: int }

function makeTokenizer(source: &string): @lifetime("source") Tokenizer {
    return Tokenizer { source, pos: 0 };
}
```

**Verification:**

The compiler verifies that the annotation is correct. Returning a value that doesn't
actually borrow from the declared parameters is a compile error:

```ds
function wrong(a: &string, b: &string): @lifetime("a") &string {
    return b;  // ERROR: return borrows from 'b', not 'a'
}
```

**Call site tracking:**

At call sites, the compiler uses lifetime information to check safety:

```ds
function caller(x: &string): &string {
    const local = "hello";
    return first(x, &local);  // OK: first only borrows from 'x'
}

function bad(): Tokenizer {
    const local = "hello";
    return makeTokenizer(&local);  // ERROR: Tokenizer borrows from local
}
```

This design provides precise borrow tracking without requiring Rust-style `<'a>`
lifetime parameters on every function signature.

### Ownership and using
In `.ds` files, `^T` values are cleaned up at their last proven use.
`using` pins cleanup to scope exit even if the value would otherwise be cleaned up earlier.
This pinning remains lexical even when the scope contains `await` or `yield`.
`using` bindings are not movable, so `^x` from a `using` binding is an error.
Use `^T` without `using` when you want eager cleanup.

Ownership and `using` are intentionally separate mechanisms:
- `^T` controls memory ownership and lifetime.
- NLL cleanup for `^T` is compiler inserted.
- `using` controls resource protocol disposal through `Disposable` or `AsyncDisposable`.
- `using` does not require `^T`, and `^T` does not imply `Disposable`.

```ds
using buffer: ^Buffer = allocate();
use(&buffer);
```

## Comptime

It is often useful, especially in statically compiled languages, to denote some expression as evaluatable or evaluated at "compile time".
There are many ways of doing this (hello `constexpr`), but we find Zig's `comptime` concept to be a natural fit to TypeScript's system (with some modifications).

Destack supports `comptime` as a modifier on bindings like `T<comptime N>` to denote that the type `T` is a (value-space) value at compile time, or as an expression form that executes the expression during compilation like `comptime <expression>`.
See [Types](#types) for static parameter declarations and constraints.
See [Targets](#targets) for profile context used during comptime evaluation.

### Static vs Dynamic Comptime

To keep language (and compiler) semantics sane, there are two distinct notions of "compile time"; the distinction basically centering around _when_ during compile time a value is evaluated:
- **Static comptime execution**: Static parameters like `E` and `N` in `type FixedArray<E, comptime N: int> = E[N]` must be known statically _during analysis_, we require static parameters to be evaluatable statically using a powerful but restricted set of expressions.
- **Dynamic comptime execution**: Comptime _expressions_  like `let precomputedTable = comptime { ... }`, on the other hand, support the full "comptime world" and basically all expressions. These are executed post-analyze in topological order (no cycles) and then patched into the IR.


### Import Meta

`import.meta` exposes per-profile metadata during static and comptime evaluation.
The values are fixed for the profile and are not runtime dependent:

```ds
/// Metadata about the current module and build configuration.
export interface ImportMeta {
    /// The URL of the current module (file:// for local, https:// for remote).
    readonly url: string,
    /// The file system path of the current module (only for local files).
    readonly path: string | undefined,
    /// Alias of `path`.
    readonly file: string | undefined,
    /// Alias of `file`.
    readonly filename: string | undefined,
    /// The directory containing the current module (only for local files).
    readonly dir: string | undefined,
    /// Alias of `dir`.
    readonly dirname: string | undefined,
    /// The output format being compiled.
    readonly output: Output,
    /// The target platform (OS) being compiled for.
    readonly platform: Platform,
    /// The runtime environment that will execute the code.
    readonly runtime: Runtime,
    /// True if this is a debug/development build (from target.debug).
    readonly debug: boolean,
    /// True if this is a test build.
    readonly test: boolean,
    /// Environment variables (from build configuration).
    readonly env: ImportMetaEnv,
}
```

### Static Ifs

The `@if(...)` decorator gates declarations and declaration members based on a static comptime expression.
(The condition must evaluate to a boolean using static execution, and  must not depend on static parameters.)
When the condition is false, the annotated item is (conceptually) removed from the symbol table for the active profile.
Multiple `@if` decorators are combined with logical AND.
The decorator is allowed on module declarations, class and struct members, interface members, and enum fields.

```ds
enum OperatingSystem {
    @if(import.meta.platform == "windows")
    Windows,
    @if(import.meta.platform == "macos")
    Mac,
}
```

### Comptime Expressions

The `comptime` keyword wraps an expression or block, forcing compile-time evaluation.
Comptime expressions infer their result type like any other expression, though it is recommended to annotate the type explicitly.

```ds
const VALUE: int = comptime 1 + 2 + 3;
const RESULT: int = comptime factorial(10);
const TABLE: uint8[] = comptime {
    let t = [];
    for (let i = 0; i < 256; i++) {
        t.push(computeCRC(i));
    }
    t
};
```

### Comptime Blocks

Comptime blocks can appear as struct/class members or at module top-level.
Unlike static blocks (inherited naming from TypeScript), comptime blocks run during compilation rather than at initialization time.

#### Member-level Comptime Blocks

Comptime blocks inside structs or classes run once per instantiation of the type at compile time.
They are useful for compile-time assertions on static parameters:

```ds
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0, "buffer size must be positive");
        assert(size <= 65536, "buffer size too large");
    }

    data: uint8[size],
}
```

Member comptime blocks have no return value (they evaluate to `void`).
They execute after the type's static parameters are resolved but before any instances are created.

#### Module-Level Comptime Blocks

Module-level comptime blocks run during module compilation:

```ds
comptime {
    // validation or initialization logic
    assert(TARGET_ARCH == "x64" || TARGET_ARCH == "arm64");
}

// or to compute a value
const LOOKUP_TABLE: uint8[256] = comptime {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

### Static Parameters

Static parameters support both type parameters and (static) comptime value parameters.
By default, static parameters like `<T>` are type parameters like in TypeScript, so comptime value parameters must be marked unambiguously with `comptime`.

```ds
function repeat<comptime N: int>(value: string): string {
    let result = "";
    for (let i = 0; i < N; i++) {
        result += value;
    }
    result
}

const greeting = repeat<3>("hello ");  // N is comptime
```

### Comptime Dynamic Parameters

Dynamic parameters can also be marked `comptime` to require compile-time-known arguments:

```ds
function createBuffer(comptime size: int): uint8[] {
    let buf: uint8[size] = [];
    for (let i = 0; i < size; i++) {
        buf[i] = 0;
    }
    buf
}

createBuffer(1024);           // ok: literal is comptime-known
createBuffer(config.size);    // error: config.size not comptime-known

const N = comptime 1024;      // ok: redundant but valid
createBuffer(N);              // ok: N is comptime-known
```

This works exactly like static parameters.

### Comptime Conditions

`if (comptime ...)` evaluates the condition as a static expression.
There is nothing really special about comptime conditions, they just dead code eliminate like any statically evaluatable expression.
Type relations like `T extends U` are valid inside comptime conditions, which is quite useful.
However, both branches must type check even when the condition is statically known (unless statically excluded with `@if`).
}

## Reflection

Destack supports reflection of `type`s as first-class values, i.e., you can introspect the fields of a type.
See [Annotations](#annotations) for decorator syntax and annotation attachment points.
See [Modules](#modules) for schema and metadata related imports.

### Type Descriptors

Every nominal type `T` has a corresponding runtime descriptor value of type `Type<T>`.
For classes and structs, the constructor value doubles as the descriptor, so it is both constructable and reflective.
Using a type name in value position evaluates to that descriptor value:

```ds
struct Point {
    x: float32;
    y: float32;
}

const t = Point;              // t: Type<Point>
const t: Type<Point> = Point; // explicit annotation
```

The `typeOf` function returns a descriptor for a value's type (unlike the runtime `typeof`, which returns a coarse-grained string like `"object"`).
The type-level `typeof` operator returns the value type of an expression, including constructor signatures and static members for classes and structs:
The `typeof` operator is type-only, and type aliases are not values in expression position.

```ds
const p = Point { x: 1, y: 2 };
const t = typeOf(p);          // Type<Point>
```

### Decorator Metadata

Decorator information is accessible at runtime on-demand:

```ds
@deprecated("use newAPI")
function myOldMethod() {}

myOldMethod.decorators  // [{ name: "deprecated", args: ["use newAPI"] }]
```

### Standard Library

The built-in `Type<T>` interface provides basic reflection.
The standard library `@destack/schema` extends it for general schema use:

```ds
// built-in (always available with Reflection feature)
Point.name        // "Point"
Point.fields      // [{ name: "x", ... }, { name: "y", ... }]
Point.is(value)   // type guard

// standard library (requires import)
import { parse } from "@destack/schema";
parse(Point, data)      // runtime validation
Point.parse(data)       // shorthand via extension
```
