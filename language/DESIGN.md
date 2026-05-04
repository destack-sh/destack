# "Language"

The Destack language (`.ds`) and toolchain, colloqially "TypeScript++", are a superset of "strict modern" TypeScript with support for `.ts` and `.tsx` files, native AOT compilation and a fully integrated language toolchain, _and_ it can also compile nicely to standard JS/TS targets.
We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated stack (Destack), and thus by "language" ("TypeScript++") we mean much more than "just" a syntax form: a language, a runtime, a toolchain, plugins, and ultimately, a way of programming.

## "TypeScript++"

We're very early in software, and we're still figuring out how to build optimal, correct, and integrated software systems.
Over 50 years, we have grown more and more layers of software sediment and need ever _more_ tools to manage the get any code out the door, and yet confidence and performance have plummeted.

We believe the best possible stack is the most integrated one, and it must truly span the entire lifecycle: the language itself, the toolchain with linters and formatters, a VM, compiler, runtime, and basically anything that touches the code.
Only TypeScript is seriously close to being a universal software foundation, because it runs direclty on the web, and the web is the most ubiqutious application platform.
The TypeScript ecosystem has good - if not perfect - answers to all modern software needs, from great developer tools to rich interactive frontends to quite _decent_ and performant backends.

If you remove all the JS baggage and dynamic prototype mess, modern TypeScript is surprisingly close to a fully AOT-compilable language (and most browsers retrofit compilation internally already based on this assumptions).
Embracing TypeScript and "the web ecosystems" lets us build a new toolchain that truly covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be completely free of JS overhead and (some) historic baggage.

## Compatibility

**Destack aims to accept modern strict TypeScript that already has static shape**.
That is the useful subset anyway.
If you want the full dynamic JavaScript object model, TSC and existing JS engines already exist.
Destack is for TS-shaped code that can be type checked, optimized, and compiled ahead of time without pretending every value might turn into a different shape at runtime.
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
- **`any`**: Portable `.ds` uses `unknown` as the top type.
  TypeScript `any` is rejected because it makes arbitrary property access, calls, and assignments appear valid without proof.
  Existing TS code must narrow through `unknown`, use explicit casts at interop boundaries, or stay outside portable Destack.
- **XML namespace resolution**: Destack does not implement XML `xmlns` namespace binding semantics.
  Namespaced tree tags like `<svg:path />` are treated as intrinsic string tag names (`"svg:path"`).

### Shapes

Dynamic shapes and strict native compilation do not mix.
In `.ds`, values have statically known shape, and classes have a fixed static object model instead of some mutable JS constructor object.

- **Declaration expressions**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with proper AOT compilation.
- **Dynamic code generation**: Runtime `eval`, runtime `new Function`, and dynamic class generation are in conflict with a strict AOT model and unsupported.
  We do however support `comptime` forms _during_ compilation for certain use cases.
- **Prototype objects**: `.prototype`, `.__proto__`, `.constructor`, `Object.getPrototypeOf`, `Object.setPrototypeOf`, and `Object.create(proto)` all rely on the prototype-based object model and are not supported.
- **Shape mutation**: `delete`, `Object.defineProperty`, `Object.defineProperties`, `Reflect.defineProperty`, `Reflect.deleteProperty`, and shape-changing `Object.assign` are forbidden.
- **Metaobject dispatch**: `Proxy` and most `Reflect.*` APIs exist to intercept or emulate dynamic object behavior, so they are also unsupported.
- **CommonJS mutation**: `require`, `module.exports`, and require-cache monkeypatching are dynamic module-shape features, and are also (mostly) unsupported.

### Protocols

JS also has a lot of behavior where the runtime secretly calls user code through special names or symbols.
Destack instead uses typed protocols, declared members, static members, and extensions instead.

- **Thenables**: `await` does not mean "anything with a `.then` property".
  It targets `Promise<T>` or another typed async protocol.
- **Coercion hooks**: `valueOf`, `toString`, and `Symbol.toPrimitive` do not participate in implicit object coercion.
  Use explicit conversions, formatting/display protocols, interpolation, or operator overloads.
- **Loose equality coercion**: Object coercion through `==` and `!=` is not part of portable `.ds`.
- **Well-known symbol magic**: `Symbol.hasInstance`, `Symbol.species`, `Symbol.isConcatSpreadable`, and similar hooks are not language semantics.
  Iteration can still exist as a typed `Iterable<T>` protocol, even if a JS target lowers it to symbols.
- **Implicit call and construct hooks**: Arbitrary `[[Call]]`, `[[Construct]]`, and `Function.prototype.call` / `apply` / `bind` are not implicit members.
  Callable and constructable values must have declared callable or constructable types.

# Language

"TypeScript++" is a superset of "strict modern" TypeScript, which essentially means that existing TypeScript _just works_ **if** it follows our strict TypeScript-based type system.
Fortunately, strict TypeScript is already a best practice, and it's what you get when enabling the recommended soundness flags in TSC.
TypeScript++ then adds new features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do for frontend-shaped softare, but for the entire software stack including "systems software".

There are solid arguments that a language should be minimal (like Zig or Go or even C), but we do not believe "language minimalism" to be pragmatic for the universal language and toolchain we want.
That said, TypeScript is already not a simple language, and any additional language features risk becoming unwieldy.
We needed _some_ additions for serious systems programming, and we wanted to take the opportunity to round out the language with modern ergonomics like patterns, operator overloading, reflection, and comptime.

| Feature | What | Why |
|---------|-------------|-----|
| [**Types**](#types) | Type system extensions: primitives, nominality, tuples, generic values, associated types, reflection | Soundness, memory, precision |
| [**Expressions**](#expressions) | Expression extensions: blocks, patterns, trees, errors, operators, dispatch, decorators, comptime | Better ergonomics |
| [**Memory**](#memory) | Ownership, borrowing, local/shared spaces, and memory type algebra | Systems programming |
| [**Runtime**](#runtime) | Globals and typed imports for code, data, text, and binary assets | Program assembly |

## Types

Destack extends TypeScript's type system with precise primitives, nominal types ("`newtype`s"), value types ("`struct`s"), tuples, ergonomic constraints, and some additional niceties.

### Primitives

TypeScript inherits its primitive types from JavaScript: `object`, `string`, `boolean`, `number`, `bigint`, and `symbol`, plus the `null` and `undefined` sentinels.
Destack adds precise numeric types beyond `number` with variable width signed and unsigned integers (`int8`, `uint32`, `int17`) as well as single and double precision floats (`float32`, `float64`).
`number` is just an alias to `float64`.

```ds
const id: uint64 = 12345;
const balance: float32 = 100.50;
const n: number = 1.0;
n satisfies float64;
```

Pointer-sized integers - that integers that are as wide as the target's pointer size - are spelled `isize` and `usize`, respectively.

### Newtypes

TypeScript is structurally typed, that is, an interface is satisfied by any value matching its shape, even when it doesn't explicitly `implement` it (similar to Go).
Structural typing is a useful default, but sometimes explicit nominality is important for correctness and expressiveness.
Destack adds `newtype` as anominal counterpart to `type`: newtype aliases and newtype interfaces, which are really just a convenience around newtype aliases.

With plain `type`s and aliases, there is no actual protection against accidental assignment.
(The TS ecosystem commonly resorts to "branding" hacks to work around this limitation.)
```ts
type UserId = number;
type OrderTag = string;

0 satisfies number; // OK, TS is happy, but ouch
"invalid" satisfies OrderTag; // OK, TS still happy, also ouch
```

With explicit `newtype`, receiver types must be explicitly cast into the nominal form:
```ds
newtype UserId = number;
newtype OrderTag = string;

0 satisfies number; // ERROR!
"invalid" satisfies OrderTag; // ERROR!
```

To actually cast a value to a newtype you either use regular `<expr> as T` conversion or explicit `T(..)` style construciton, like:
```ds
newtype UserId = number;
newtype OrderTag = string;
newtype Point = (number, number);
newtype Rectangle = {
    start: Point,
    end: Point,
}
newtype AuthenticatedUser = User;

UserId(1) satisfies UserId;
OrderTag("tag") satisfies OrderTag;
Point(1, 2) satisfies Point;
Rectangle { start: ..., } satisfies Rectangle:
AuthenticatedUser(user) satisfies AuthenticatedUser;
```

### Newtype Interfaces

Newtype aliases add nominality to any type, and Destack thus also supports **nominal interfaces** using the `newtype` modifier on `interface` declarations:

```ds
// structural interface (standard TypeScript behavior)
interface Drawable {
    draw(): void;
}
const x: Drawable = { draw() {} };  // OK: structural match

// nominal interface (requires explicit `implements`)
newtype interface Add<T, R = this> {
    add(other: T): R;
}
```

Nominal interfaces require **explicit `implements`** declarations - structural compatibility alone doesn't satisfy the constraint, unlike for regular `interface`.
Nominal interfaces are used for operator interfaces like `Add` and `Compare`, and for capability traits like `Send`, `Sync`, `Copy`, and `Clone`.

### Extensions

It is sometimes convenient to attach additional logic and data to the (nominal identity of) a type.
Rust supports this with `impl` blocks (and only `impl` blocks, actually), and Destack supports _additional_ `extension`s to add methods and static constants to any _nominal_ type:

```ds
newtype Vector2 = {
    x: float32;
    y: float32;
};

extension of Vector2 {
    static ZERO = Vector2 { x: 0.0, y: 0.0 };

    magnitude(): float32 {
        return (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

Extensions can be added to any **nominal types**, so all types like `struct`, `class`, `enum`, `newtype`, whether defined locally or in a foreign / imported module.
Accordingly, plain type aliases (`type X = ...`) and structural types (`{ x: number }`) cannot receive extensions (because it would be unclear when they should apply).
Further, extensions can be named for explicit exports and subsequent imports:

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

Enums are nominal aliases to a set of constants, just like in Typescript, except that Destack's enums do not implicitly cast to with their backing type. 
Explicit conversions are required when you want the backing value.
Like other nominal types, enums can carry static members and methods and can also receive extensions.

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


### Structs

Structs are nominal value types for data with fixed shape, but without reference identity, constructors, or inheritance.
Basically, structs are just data with a name, much like structs in other "systems languages": an alias to the struct's components.

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

### Arrays, Slices and Tuples

Destack supports richer sequence forms bBeyond the classic dynamic arrays - `T[]` / `Array<T>` with explicit slice, fixed array, and tuple forms:

| Forms | Meaning |
|------|---------|
| `T[]`, `Array<T>` | Dynamic, homnogenous, dense array |
| `[T]`, `Slice<T>` | Fixed, homogenous slice into dense array |
| `[T; N]`, `FixedArray<T, N>` | Fixed, owned sequence of values |
| `(A, B)` | Sequence of heterogenous, owned values |

Unlike JavaScript, Destack does not permit holes in arrays (or any other sequences)
Indexing into `T[]` therefore returns `T`, not `T | undefined`; out-of-bounds indexing traps or errors according to the active profile.

```ds
let xs: int32[] = [1, 2, 3];
let ys: Array<int32> = [1, 2, 3];
```

Fixed arrays are homogeneous arrays whose length is statically known and part of the type.
They are inline value/layout types by default, and definitionally cannot grow.

```ds
type Block = [uint8; 4096];
type Vec3 = [float32; 3];

let rgb: [uint8; 3] = [255, 128, 0];
let zeroes: [uint8; 32] = [0; 32];
```

For tuples, we still parse the "array tuple" syntax like `[number, string]` in non-`.ds` files, but require explicit tuple syntax like `(number, string)` in `.ds`.
Tuples are fixed heterogeneous products, and of course also work as patterns:

```ds
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

One-element tuples use a trailing comma, empty tuples are just `()`:

```ds
type One = (int32,);
const one: One = (1,);
const empty: () = ();
```

### Generics

Destack keeps TypeScript-shaped generics and extends generic parameter lists with compile-time value parameters.
Type parameters still use TypeScript-style inference, constraints, defaults, conditional types, mapped types, and indexed access types.
Compile-time value parameters are ordinary generic parameters whose values are known during static evaluation.

```ds
type Buffer<comptime N: uint> = [uint8; N];

function repeat<comptime N: uint>(value: string): string {
    let result = "";
    @unroll(N)
    for (let i = 0; i < N; i++) {
        result += value;
    }
    result
}
```

### Associated Types and Constants

Associated types and constants contribute static members to a type that can be reused within the type and implementors but does not need to be exposed to every single caller.
Both associated types and constants also work in abstract types, in much the way we would expect.

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

Associated types can have their _own_ generic parameters with the same generic parameter forms as ordinary declarations, including type parameters and `comptime` value parameters.
That matters for APIs where the implementor chooses a whole type family, not just one output type.

```ds
interface Storage {
    type Handle<T>;
}

struct SharedStorage implements Storage {
    type Handle<T> = shared StorageHandle<T>;
}
```

In addition to associated types, nominal type declarations also support associated constants as static compile-time values.
Unlike `static` members, `comptime const`s require no instance storage and are statically evaluated during compilation.

```ds
interface RegisterBlock {
    comptime const Width: uint;

    read(): [uint8; this.Width];
    write(bytes: &[uint8; this.Width]): void;
}
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
Destack supports type reflection both at runtime and at compile time with opaque handles of type `Type<T>`.
A type expression can be turned into its handle with (implicit or explicit) casting to its `Type` representation:

```ds
struct User {
    name: string;
    age: uint;
}

let u: User = User { name: "Alice", age: 30 };

const UserType: Type<User> = User;
const UserType = Type.of<User>();
displayNameOf(UserType) // "User"
shapeOf(UserType) // ReferenceType or ObjectType, depending on the normalized type
```

This also works for generic APIs that operate on types as static values:

```ds
function parse<comptime T: Type>(raw: string): T {
    ...
}

const user = parse<User>("...");
```

The reflection shape is a stable source-facing view of the normalized type, broadly aligned with DIR type families like literals, primitives, references, objects, functions, arrays, tuples, unions, intersections, conditionals, mapped types, and storage forms.
It is not the compiler IR.
Compiler plugins may eventually expose AST / DIR / MIR handles directly, but ordinary reflection stays small enough to use in programs.

Beyond semantic shape queries, type reflection also supports classic layout queries like `sizeOf<T>()`, `alignOf<T>()`, `strideOf<T>()`, and `layoutOf<T>()`.
Layout queries are target/profile-sensitive and are intentionally separate from `shapeOf`.

```ds
const userSize = comptime sizeOf<User>();
const requestLayout = comptime layoutOf<Request<Body>>();
type InlineBytes<T> = [uint8; sizeOf<T>()];
```

## Expressions

### Values

In TypeScript, control flow expressions like `if` are a statement, and you need a ternary or temporary to get a value out.
That's fine but makes match-like patterns more complex to express, so Destack also supports statements as expressions ("everything is an expression").
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

### Patterns

TypeScript already has pattern based destructuring for arguments and assignment-like expressions, so match and richer pattern expressions fit in quite naturally.
Destack supports `match` with full pattern matching and exhaustiveness checking:

```ds
match (result /* Result<T, E> */) {
    { kind: 'ok', value } => process(value)
    { kind: 'err', error } if (isRetryable(error)) => retry()
    { kind: 'err', error } => fail(error)
}
```

As one would expect, the type of a match expression is the union of its case body types, and patterns can appear in `match`, `if let`, `let ... else`, and destructuring bindings.
The core pattern families are wildcard, binding, literal, tuple, object/struct, array/slice, variant/newtype, union, and guarded patterns.

```ds
declare point: Point;
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at ${x}, ${y}` // irrefutable if point: Point
}
```

Some patterns are irrefutable, that is, they always match, and then we don't need any alternative branches, like with `_` or destructuring of known shapes.
Refutable patterns require some fallback such that all branches are covered: a `match` fallback arm, an `else` branch for `if let`, or an `else` continuation for `let ... else`.

```ds
declare point: Point;
match (point) {
    Point { x: 0, y } => "vertical"
    Point { x, y: 0 } => "horizontal"
    _ => "neither" // required fallback
}

declare maybe
let Some(value) = maybe else {
    return Result.err("missing value");
};
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
```

### Operators

Destack extends TypeScript operators with typed overloads and some additional precision.
Logical operators (`&&`, `||`, `??`), optional chaining, assignment, and strict identity (`===`, `!==`) are not (directly) overloadable, as usual.

| Operator | Example | Interface |
|----------|---------|----------|
| `+` | `a + b` | `Add<T, R>` |
| `-` | `a - b` | `Subtract<T, R>` |
| `*` | `a * b` | `Multiply<T, R>` |
| `/` | `a / b` | `Divide<T, R>` |
| `%` | `a % b` | `Remainder<T, R>` |
| `**` | `a ** b` | `Power<T, R>` |
| `+` | `+a` | `Plus<R>` |
| `-` | `-a` | `Negate<R>` |
| `&` | `a & b` | `And<T, R>` |
| `\|` | `a \| b` | `Or<T, R>` |
| `^` | `a ^ b` | `Xor<T, R>` |
| `~` | `~a` | `Not<R>` |
| `<<` | `a << b` | `ShiftLeft<T, R>` |
| `>>` | `a >> b` | `ShiftRight<T, R>` |
| `>>>` | `a >>> b` | `ShiftRightUnsigned<T, R>` |
| `==`, `!=` | `a == b` | `Equal<T>` or `PartialEqual<T>` |
| `<`, `<=`, `>`, `>=` | `a < b` | `Compare<T>` or `PartialCompare<T>` |
| `[]` | `a[i]` | `Index<I, O>` |
| `[] =` | `a[i] = v` | `IndexSet<I, V>` |
| `*` | `*a` | `Deref<T>` |
| `* =` | `*a = v` | `DerefSet<T>` |

When overflow / wrapping policy is part of the algorithm, the expression should say so directly, so Destack provides Zig-style wrapping and saturating arithmetic for integer code:

| Operation | Standard | Wrapping | Saturating |
|-----------|----------|----------|------------|
| Add | `a + b` | `a +% b` | `a +\| b` |
| Subtract | `a - b` | `a -% b` | `a -\| b` |
| Multiply | `a * b` | `a *% b` | `a *\| b` |

For example, with `a: uint8 = 250` and `b: uint8 = 10`:

| Operation | Standard | Wrapping | Saturating |
|-----------|----------|----------|------------|
| `a + b` | trap / error (overflow) | `4` (wraps past `255`) | `255` (clamped to max) |
| `a - 255` | trap / error (underflow) | `251` (wraps below `0`) | `0` (clamped to min) |
| `a * b` | trap / error (overflow) | `196` (wraps modulo `256`) | `255` (clamped to max) |

The explicit forms ignore the safety profile.
`+%`, `-%`, and `*%` wrap modulo the integer's range.
`+|`, `-|`, and `*|` clamp to the integer's minimum or maximum value.

```ds
const hash = (hash *% 16777619) +% byte;
const volume = left +| right;
```

Overflow-policy operators are not overloadable.
For user-defined numeric types, use ordinary named methods when wrapping or saturation is part of the type's API.

### Dispatch

"Dispatch" is how calls, member accesses, and overloadable operators select an implementation to invoke.
The selection rule is TS-derived: build the candidate set, keep candidates compatible with the arguments as written, then pick the first one in declaration order:
 - Declaring module owns the overload order.
 - Overloads are truly distinct implementations.
 - Union arguments do not distribute across overloads (one selected overload must accept the union)

```ds
function parse(input: string): int32 {
    return parseInt(input);
}

function parse(input: int32): int32 {
    // legal, actually different implementation
    return input;
}
```

Members work the same way (after receiver lookup): inherent members first, then visible extension members in declaration order.
For overloadable operators, the token chooses the protocol, the left operand is the receiver (matching how it is written in the interface implementation).

```ds
newtype interface Add<T, R = this> {
    add(other: T): R;
}

extension of Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}

extension of Vector2 implements Add<float32> {
    add(other: float32): Vector2 {
        Vector2 { x: this.x + other, y: this.y + other }
    }
}

const moved = position + offset; // Add<Vector2>
const padded = position + 1.0;   // Add<float32>

moved satisfies Vector2;
padded satisfies Vector2;

1.0 + position; // requires Add<Vector2> on float32
```

Union receivers are resolved per variant.
Every variant must expose the member; if all variants resolve to the same symbol the call is static, otherwise the compiler records a dynamic dispatch and the result type is the union of the selected return types.

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
However, Destack also supports and strongly encourages **Result-first error handling** inspired by Rust: recoverable errors use `Result<T, E>`, nice try-catch integration, and even support for `?` and `??` coalescing.

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

We typically construct results through `Result.ok(value)` and `Result.err(error)`:

```ds
function readConfig(path: string): Result<Config, IOError> {
    const text = readFile(path)?;    // propagate errors with ?
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

When applied to `Result<T, E>`, `?` unwraps `Ok<T>` and propagates `Err<E>`.
The `??` coalescing operator then also supports a convenient default value for the fallback case:

```ds
const config = loadConfig() ?? defaultConfig;  // use default on error
```

#### Try

`?` and `??` are extensible operators via a standard `Try` newtype interface (which `Result<T, E>` implements, jus tlike any userland type, and also basically like in Rust). 
The `Try` trait is quite simple, we just define the success branch with `TryContinue<T>` and the failure branch with `TryFailure<E>`, like this:

```ds
type TryContinue<T> = { kind: "continue"; value: T };
type TryFailure<E> = { kind: "failure"; error: E };
type TryBranch<T, E> = TryContinue<T> | TryFailure<E>;

newtype interface Try {
    type Value;
    type Error;

    branch(): TryBranch<this.Value, this.Error>;
}
```

For `Result<T, E>`, `Ok { value }` branches to `TryContinue<T>` and `Err { error }` branches to `TryFailure<E>`.
The branch names describe the operator's control flow, not the data constructors of any one type.

Propagation is a separate target-side operation.
When a failure leaves the current function, the enclosing return type must implement `FromFailure<E>` for the propagated error type.

```ds
newtype interface FromFailure<E> {
    static fromFailure(error: E): this;
}
```

Extending from TypeScript, `?` unwraps one success layer or propagates one failure layer.
Outside a `try` block, propagation returns from the enclosing function using the return type's `FromFailure` implementation.
Inside a `try` block with `catch`, propagation transfers the failure value to the catch instead.
That means the source value needs `Try`, while the enclosing return type only needs `FromFailure` when the failure actually escapes.

`??` keeps TypeScript's nullish behavior for ordinary nullable values.
For `Try` values, it behaves like generalized coalescing: failure uses the fallback, success unwraps the value, and a nullish success value also uses the fallback.
Mixed `Try | null | undefined` inputs are allowed, but arbitrary `Try | NonTry` unions must be narrowed first.

#### Try, Catch and Finally

The `try`/`catch` syntax handles both exceptions and explicit `Try` propagation:

```ds
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e: IOError) {
    log("Failed to read config:", e)
}
```

The example uses `Result`, but any type implementing `Try` behaves the same:
- Note that `try` does not implicitly unwrap `Result` values
- Use `?` inside the block to propagate `Try` failures into the catch
- Use `??` inside the block when the failure should be handled locally with a fallback
- When a `?` is inside a `try` with a catch, `FromFailure` is not required

When just using known `Try`-implementor types, the full set of possible failures is known, and we can use a syntax sugare form called `catch match` to branch on them directly:

```ds
try {
    riskyOperationA()?; // -> Result<void, AError>
    riskyOperationB()?; // -> Result<void, BError>
} catch match (e /* AError | BError */) {
    NumericError(x) => Error(`bad number: ${x}`)
    FormatError => Error(`bad format ${e}`)
    _ => Error(`unknown error: ${e}`)
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
Essentially, `TreeTag` generalises `jsxFactory` and `TreeTagBuilder` generalises `jsxFragmentFactory`: 
 - Uppercase or qualified tags resolve as value tags through normal value lookup and the `TreeTag` interface.
 - Lowercase unqualified tags resolve as intrinsic tags through the active `TreeTagBuilder`.

### Annotations and Decorators

Like TypeScript, Destack uses `@` for decorators. 
Unlike in TypeScript, Destack decorators can appear basically on any declaration, item and statement, much like Rust attributes.

```ds
@deprecated("use newAPI instead")
function oldAPI() {
    // ...
}

@unroll
for (let i = 0; i < 4; i++) { }

@derive(Clone, Serialize, Reflect)
struct User {
    id: UserId;
    name: string;
}

function kernel(data: @space("shared") &Point) { }

match (result) {
    @cold
    Err { error } => handleError(error),
    Ok { value } => value,
}
```

#### Static If

There is a another special decorated: `@if` gates the inclusion of certain nodes based on some statically evaluatable expression.
When the condition is false, the annotated item is (in effect) removed and removed from analysis and the final shape.

```ds
enum OperatingSystem {
    @if(import.meta.target.os == "windows")
    Windows,
    @if(import.meta.target.os == "macos")
    Mac,
}
```

`@if` works on module declarations, class and struct members, interface members, enum fields, and other declaration-shaped nodes.

### Globals

TypeScript supports ambient global typings, which were designed for typing the "magic" global objects provided by embedders, but there is no way to contribute _value_ globals in userland, which is sometimes quite convenient.
In addition to ambient global typings, Destack therefore supports "real" `global { ... }` value declarations that contribute to the ambient environment without explicit imports.

```ds
global { // just omit the `declare`!
    const console: Console = runtime.console();
    const runtimeId = Runtime.current.id;
    shared const registry = new Registry();
}
```

A `global` block is active when its containing module is in the closure reached from the target's entrypoints, includes, configured global provider roots, or profile/prelude modules.
Of course, because these globals are real values, duplicate global value names are errors.

### Comptime

Inspired by Zig and Jai, Destack supports compile-time evaluation with `comptime`.
The `comptime` keyword and modifier requires that an expression be evaluated at compile time (otherwise it is a compile error).

```ds
const LOOKUP_TABLE: uint8[] = comptime {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

Unlike in other languages, functions do not declare themselves as either "comptime" or "runtime"; instead, evaluation "time" is inferred from the usage site.
Functions and "comptime" functions can therefore intermingle freely and call each other:

```ds
function factorial(n: int): int {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

const COMPTIME_CONST = comptime factorial(10);    // compile time
const RUNTIME_CONST = factorial(getUserInput()); // runtime (in this case, at module initialization time)
```

Comptime conditions enable branch elimination and, for type relations like `T extends U`, type narrowing:

```ds
function process<T, Context: CacheContext<T>>(ctx: Context, key: T) {
    if (comptime Context extends EvictableContext<T>) {
        ctx.onEvict(key);  // context is narrowed; branch eliminated if not satisfied
    }
}
```

Comptime blocks can also appear as members on object-like types for `comptime` associated logic, much like `static` blocks are runtime associated logic.

```ds
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0 && size <= 65536);
    }
    data: [uint8; size],
}
```

## Memory

TypeScript, like most managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed on some local heap, and all value types are copied by default.
That is convenient, but sometimes we need to take direct control of memory, whether for better performance, or just to express invariants in the code.

Destack adds explicit, optional modifiers for controlling memory ownership and placement, inspired by Rust and Mojo with `^T` as the "owned" signifier.
Plain `T` keeps the base type's default representation: value types are values, object types are managed references.
Memory in Destack lives on two orthogonal axes:
- **Ownership**: who "owns" the value: managed (`T`), owned (`^T`), borrowed (`&T`), or raw (`*T`).
- **Placement**: where the value is located: ambient by default, `shared` across Workers, explicit `"local"` in the type algebra, or some other target-defined space.

The two axes compose freely, e.g. `^shared T` is an owned handle to a value in shared space, and `&shared T` is a borrow of a shared value.

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
let a: User = new User();         // managed handle
let b: ^User = new User();        // owned handle
let c: &User = &a;                // borrow
let d: *User = unsafe { &raw a }; // raw pointer
```

Plain `T` is the default and matches what TypeScript already does: value types are values, object types are managed references.
`^T` denotes unique ownership, like Rust's `Box<T>` or Mojo's `^T`.
`&T` is a borrow that must remain valid for some region; see [Borrows, Regions And Suspension](#borrows-regions-and-suspension).
`*T` is an unchecked raw pointer; it does not keep anything alive and can only be used inside `unsafe` blocks.

### Space

Space defines where the memory is actually located in memory, and following web standards, Destack uses `Worker`-local heap as the main memory space.
The default *local* memory space is the current Worker's local heap, and that's where ambient types land unless otherwise specified.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space, and user and library code can almost always just pretend spaces don't exist.

Often, the "space" of a type and its corresponding memory region are a purely logical separation: most computers have unified main memory, and separating local and shared (and other..) heaps is much more about corectness (and somewhat about performance) than about physical constraint.
For non-uniform memory targets, assigning specifi cmemory spaces in one unified programming language is however quite convenient.

```ds
struct Request<T> {
    header: Header,
    body: T,
}

let here: Request<Body>;          // header, body are local
let there: shared Request<Body>;  // header, body are shared
```

Memory placement is contextual and types are ambient by default: an aggregate field with ambient placement is interpreted in the placement of the containing value, while an explicit placement on a field is preserved.
Some incompatible combinations of explicit placements - like local inside shared - produce an error.
More broadly, the compiler may lower one source aggregate into distinct concrete layouts depending on its space.
This is why the type algebra distinguishes `Place` from `Space`: `Space` is concrete, while `Place` may also be `"ambient"`.

#### Shared Space

The "shared space" is shared memory visible to all `Worker`s in the same `Runtime`.
Conceptually, `shared` is the typed, generalized version of the `SharedArrayBuffer` idea with the full type system and object graphs at our disposal:
- Local values may point to shared values.
- Shared values must not point directly into a local heap.

It should be noted that shared placement - or any space placement - is **not** not a synchronization primitive in itself, and does **not** imply atomic access, locking, actor isolation, `Sync`, or anything like it.
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

The rules for who can convert into what mostly follow from two facts: references must always be valid, and shared memory must not point into a Worker-local heap.
(Raw pointers are your own dangerous business.)

| From \ To | `T` | `shared T` | `&T` | `&shared T` | `^T` | `^shared T` | `*T` | `*shared T` |
|-----------|-----|------------|------|-------------|------|-------------|------|-------------|
| `T` | - | yes | yes | no | no | no | unsafe | no |
| `shared T` | no | - | no | yes | no | no | no | unsafe |
| `&T` | no | no | - | no | no | no | unsafe | no |
| `&shared T` | no | no | no | - | no | no | no | unsafe |
| `^T` | no | yes | yes | no | - | no | unsafe | no |
| `^shared T` | no | no | no | yes | no | - | no | unsafe |
| `*T` | no | no | reborrow | no | no | no | - | no |
| `*shared T` | no | no | no | reborrow | no | no | no | - |

"unsafe" conversions require an explicit `unsafe` block, and "reborrow" from `*T` to `&T` requires an unsafe checked reborrow that asserts validity.

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

The primary typed construction path is `new`, which allocates heap storage, initializes a `T`, and produces the ownership form required by the destination type and the base type's affinity.
`new` is destination-typed: the expected type decides whether construction produces a managed or owned value, with affine base types preferring owned forms in unconstrained positions.

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

Okay, "algebra" here is just a fancy way of saying Destack supports querying and manipulating ownership and placement in its type system. 
Qualified surface forms like `^T`, `&T`, `*T`, and `shared T` are sugar over a single normalized `Form`.
Plain `T` may remain unqualified, but algebra operators treat it as managed ambient when they need a default:

```ds
newtype Form<
    T,
    O: Ownership = "managed",
    P: Place = "ambient",
    R = never,
> = unknown;
```

Each qualified surface form maps to a `Form<...>` with its specific ownership tag and placement:

```ds
User            // unqualified, interpreted as managed ambient
^User           // Form<User, "owned", "ambient">
&User           // Form<User, "borrowed", "ambient", R>
*User           // Form<User, "raw", "ambient">
shared User     // Form<User, "managed", "shared">
^shared User    // Form<User, "owned", "shared">
```

The operators are just the same few families applied to those axes.
Constructors build a form from a base type while preserving any existing placement:

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
BaseOf<^shared User> satisfies User;
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

It's important to emphasize that except for the intrinsic `Form`, all the rest is just regular TypeScript-shaped type algebra.
That means memory qualification is just ordinary type-level computation: libraries and frameworks can introspect and rewrite ownership and placement using the same generic machinery they would use for any other type, which is quite useful (and neat).

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

Data files are parsed at compile time and typed structurally:

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

config.server.host satisfies string;
config.server.port satisfies number;
config.debug satisfies boolean;
```

Types are inferred from the data:
- `null` → `null`
- `true`/`false` → `boolean`
- Numbers → `number`
- Strings → `string`
- Arrays → `T[]` (union for mixed elements: `(T | U)[]`)
- Objects → `{ key: Type, ... }` (readonly fields)

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
