# "Language"

The Destack language (`.ds`) and toolchain, colloqially "TypeScript++", are a superset of "strict modern" TypeScript with support for `.ts` and `.tsx` files, native AOT compilation and a fully integrated language toolchain, _and_ it can also compile nicely to standard JS/TS targets.
By "language" we mean much more than "just" a programming language, we mean the 

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
To enable more ergonomic data flow and particularly better pattern matching capabilities, Destack also supports statements as expressions ("everything is an expression").
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
const result = if let Some(value) = maybe {
    value
} else {
    0
};

if let (x, y) = point {
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


### Errors

Exceptions are deeply enmeshed into TypeScript, and therefore Destack supports them, too (alas).
However, Destack also supports and strongly encourages **Result-first error handling** inspired by Rust: recoverable errors use `Result<T, E>`, nice try-catch integration, and even support for `T`, `?` and `??` coalescing.

#### Result

Destack provides `Result<T, E>` as the primary error handling mechanism:

```ds
function readConfig(path: string): Result<Config, IOError> {
    const text = readFile(path)?;    // propagate errors with ?
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

The `?` operator propagates errors ergonomically using the builtin `Try` operator, similar to Rust.
When applied to the builtin `Result` type, `?` returns early with the error value if present.

TypeScript's `??` coalescing operator then supports a convenient default value for the failure case:

```ds
const config = loadConfig() ?? defaultConfig;  // use default on error
```

#### Try

`?` and `??` are driven by a standard `Try` protocol rather than being hard-coded only for `Result`.
`Result<T, E>` implements `Try`, and other library types may implement it when they have the same success-or-failure shape.

The protocol is about control flow, not about exceptions.
A `Try` value can branch into `Try.Continue<T>`, meaning evaluation continues with a value, or `Try.Failure<E>`, meaning the current path transfers control to the nearest handler or enclosing return.

```ds
type Try.Continue<T> = { kind: "continue", value: T };
type Try.Failure<E> = { kind: "failure", error: E };

newtype interface Try {
    type Value;
    type Error;

    branch(): Try.Continue<this.Value> | Try.Failure<this.Error>;
    static fromFailure(error: this.Error): this;
}
```

For `Result<T, E>`, `Ok(value)` branches to `Try.Continue<T>` and `Err(error)` branches to `Try.Failure<E>`.
The branch names describe the operator's control flow, not the data constructors of any one type.

`?` unwraps one success layer or propagates one failure layer.
Outside a `try` block, propagation returns from the enclosing function using the return type's `fromFailure`.
Inside a `try` block with `catch`, propagation transfers the failure value to the catch instead.

`??` keeps TypeScript's nullish behavior for ordinary nullable values.
For `Try` values, it behaves like generalized coalescing: failure uses the fallback, success unwraps the value, and a nullish success value also uses the fallback.
Mixed `Try | null | undefined` inputs are allowed, but arbitrary `Try | NonTry` unions must be narrowed first.

#### throw and native exceptions

Destack also supports exceptions and `throw` for compatibility with classical JS/TS and other exception-oriented ecosystems like Java and C#.
Destack still prefers `Result` for ordinary recoverable errors, especially in performance-critical code.

```ds
function assertPositive(n: int) {
    if (n <= 0) {
        throw new Error("invariant violated: expected positive")
    }
}
```

#### try/catch with Result and exceptions

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
- When a `?` is inside a `try` with a catch, `Try.fromFailure` is not required

Thrown exceptions propagate into the catch in the usual way when `throw` is enabled.
With `noExceptions`, `throw` is unavailable, but `try` and `catch` still work for `Result` and other `Try` values.
As usual, a `try` expression must include a `catch` or `finally` block.

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

### Operators

Destack keeps TypeScript's operators where they already have clear JavaScript semantics, and adds typed overloads only where the operator maps cleanly to an explicit protocol.
Operator overloading is receiver-based and nominal: a type must explicitly implement the corresponding nominal operator interface.

#### Operator Dispatch

For overloadable binary operators, the left operand selects the implementation family and the right operand selects the overload within that family.
For example, `a + b` lowers to the left receiver's `add` implementation when the receiver implements `Add`.

```ds
newtype interface Add<T, R = this> {
    add(other: T): R;
}

extension of Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

#### Special Operators

Some operators are deliberately not overloadable because their control-flow or type-system behavior is too fundamental.
This includes `&&`, `||`, `?.`, `as`, `satisfies`, `typeof`, `keyof`, `extends`, `implements`, `is`, `instanceof`, `?`, and `??`.

`?` and `??` are protocol-driven, but they are not ordinary overloadable operators.
Their semantics are fixed by the language and implemented through `Try`.

#### Arithmetic And Checks

Precise integers use defined overflow behavior.
The ordinary arithmetic operators follow the active safety profile.
In checked profiles, integer overflow traps.
In unchecked profiles, integer overflow wraps in two's complement.

Destack also reserves explicit wrapping and saturating forms for code that wants the policy at the expression site:

| Standard | Wrapping | Saturating |
|----------|----------|------------|
| `+` | `+%` | `+|` |
| `-` | `-%` | `-|` |
| `*` | `*%` | `*|` |

### Overloads

Real function and method overloading with distinct implementations:

```ds
function parse(input: string): int32 {
    return parseInt(input);
}
function parse(input: int32): int32 {
    return input;
}
```

Following TypeScript, and to avoid ambiguity, Destack uses **declaration order** overloading: the first matching overload wins.
The overload order is defined by the declaring module and is forwarded unchanged across exports and reexports (so the declaring module decides).

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant:
- If all variants resolve to the same symbol, the call is static.
- If the symbols differ, the compiler records a dynamic resolution and reifies it into `if (receiver is Type)` branches.

Dynamic resolution only applies when every union variant exposes the member.
Arguments must satisfy all candidate signatures, and the resulting type is the union of per-candidate return types after substitutions.
Extension methods participate in member resolution, too.

### Trees (TSX)

Destack generalizes TSX syntax for any tree-shaped data:

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

Destack's tree literals work with any tree-compatible type, not just UI component systems, and not just any _single_ JSX/TSX-style per project.
Because we have real type analysis you can mix and match.
Types can opt into custom tree tag behavior by implementing the `TreeTag` interface, and custom intrinsic types (lowercase tags like `<div>`) are programmable via `TreeTagBuilder`.

Tree literals have two routing paths.
Uppercase or qualified tags resolve as value tags through normal value lookup and the `TreeTag` interface.
Lowercase unqualified tags resolve as intrinsic tags through the active `TreeTagBuilder`.

Fragments route through the active builder's fragment type.
Namespaced tags like `<svg:path />` are not XML namespace bindings; they are intrinsic string tag names.

### Annotations And Decorators

Destack uses `@` for resolved annotations.
The expression after `@` must resolve in the current static context.
If it does not resolve, that is an error.
There is no stringly metadata by spelling alone.

```ds
@deprecated("use newAPI instead")
function oldAPI() { }

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
    Err(e) => handleError(e),
    Ok(v) => v,
}
```

The resolved expression decides the effect.

`@derive(...)` is an intrinsic expansion annotation.
It only applies to nominal type declarations.
Each argument resolves to a nominal capability/interface.

```ds
@derive(Clone)
struct User {
    id: UserId;
}
```

For `@derive(Clone)` on `User`, the compiler asks the active derive implementation for `Clone` and `User` to produce ordinary declarations, inserts those declarations, and then type checks the result normally.
Derive output is additive and declaration-shaped, usually extensions or implementations.
It does not rewrite the annotated type.
The provider API is part of the compiler/plugin surface, not runtime type reflection.

If `@expr` resolves to a supported rewrite function, it is a rewrite annotation.
That gives plain `@memoize` and configured `@memoize({ maxEntries: 256 })` the same shape: both resolve to something the compiler can call during expansion.
Rewrite annotations are narrow compiler-visible rewrites of the annotated target.
They must preserve the public type contract and may only target declaration forms they explicitly support.
Something like `@memoize` can be expressed this way, but it is not a JavaScript property-descriptor decorator and it does not mutate prototypes.

If `@expr` resolves to any other statically known value, it is metadata.
For example, `@validate(minLength(1))` may just construct a typed value that is attached to the field and later consumed by `@derive(Validate)`, a linter, reflection, or a framework.
Metadata annotations do not generate code by themselves and do not intercept reads or writes.

Annotations may be retained as reflection metadata.
Documentation comments are also exposed as normalized documentation descriptors when retained.
Full trivia belongs to AST/plugin APIs, not runtime type reflection.
Portable `.ds` does not support legacy TypeScript decorators, parameter decorators, property descriptor mutation, prototype mutation, proxies, hidden dynamic members, or arbitrary expression macros.

### Static If

`@if(...)` gates declarations and declaration members based on a static expression.
When the condition is false, the annotated item is removed before the rest of analysis can depend on it.

```ds
enum OperatingSystem {
    @if(import.meta.target.os == "windows")
    Windows,
    @if(import.meta.target.os == "macos")
    Mac,
}
```

`@if` is allowed on module declarations, class and struct members, interface members, enum fields, and other declaration-shaped nodes.
`@if` can use whatever static facts are in scope.
At module level, that mostly means `import.meta` and profile constants.
Inside a generic declaration, `@if` is evaluated after generic arguments are known, so it can also branch on those arguments, associated constants, and type algebra queries.
Multiple `@if` annotations combine with logical AND.

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

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
The `comptime` keyword, as the name implies, requires that an expression must be evaluated at compile time (otherwise it is a compile error).
Functions do not declare themselves as either "comptime" or "runtime".
Evaluation time is defined by how the expression or value is used.

```ds
const LOOKUP_TABLE: uint8[] = comptime {
    let table: uint8[] = [];
    for (let i = 0; i < 256; i++) {
        table.push(computeCRC(i));
    }
    table
};
```

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

TypeScript, like most managed high level languages, does not encode memory "ownership" in its type system: all reference types are implicitly GC-managed, and all value types are copied by default.
This is a convenient, but sometimes we need to take direct control of memory, whether for better control and performance, or just to express invariants in the code.
Destack adds explicit, optional modifiers for controlling memory ownership and placement inspired by Rust and Mojo's ownership models with `^T` as the "owned" signifier.
Plain `T` keeps the base type's default representation: value types are values, object types are managed references.

TypeScript inherits the JavaScript / web model of local, single-threaded execution.
Destack embraces and extends this ambient model and also supports more ergonomic shared memory as part of a generalized notion of "place": local to a Worker, shared across Workers in a Runtime, or in a different address space.
For shared memory, this is conceptually like a proper object graph around `SharedArrayBuffer`-like semantics, except that all object and management features work the same.

| Form | Ownership | Region | Place | Liveness | MIR shape | Value |
|------|-----------|--------|-------|---------------|-----------|-------|
| `T` for value bases | value | none | ambient | owned by its containing storage | `value<T, space(X)>` | inline value |
| `T` for object bases | managed | none | ambient | keeps the referent alive | `ref<T, managed, space(X)>` | managed heap handle |
| `shared T` for value bases | value | none | shared | owned by its containing storage | `value<T, space(shared)>` | inline shared value |
| `shared T` for object bases | managed | none | shared | keeps the referent alive | `ref<T, managed, space(shared)>` | managed shared handle |
| `&T` | borrowed | inferred or explicit | ambient | requires liveness | `ref<T, borrowed, space(X)>` | semantic borrow or projection |
| `&shared T` | borrowed | inferred or explicit | shared | requires liveness | `ref<T, borrowed, space(shared)>` | semantic shared borrow or projection |
| `^T` | owned | none | ambient | owns the referent | `ref<T, owned, space(X)>` | owned heap handle |
| `^shared T` | owned | none | shared | owns the referent | `ref<T, owned, space(shared)>` | owned shared heap handle |
| `*T` | raw | none | ambient | does not keep anything alive | `ref<T, raw, space(X)>` | unsafe raw typed pointer |
| `*shared T` | raw | none | shared | does not keep anything alive | `ref<T, raw, space(shared)>` | unsafe shared raw typed pointer |

### Local And Shared Space

Following web tradition, a `Worker` is Destack's unit of concurrent execution.
Local space is the current Worker's local heap.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space unless a type or binding says otherwise.

Shared space is runtime-shared memory visible to multiple Workers in the same Runtime.
It is the typed, generalized version of the `SharedArrayBuffer` idea rather than a separate language.
`shared T` means `T` is re-based into shared space, i.e. `WithSpace<T, "shared">`.
Other spaces can use the same type algebra, for example device or GPU memory, as long as the target and library define what values and operations are valid there.
`local` is not a surface keyword.
The local space can be named explicitly through `WithSpace<T, "local">` or `@space("local")` where an explicit space annotation is needed.
`@space(S)` on a type declaration sets that type's required placement.
This is useful for types like shared locks, device buffers, or mapped memory handles that only make sense in one space.

Local values may point to shared values.
Shared values must not point directly into a Worker-local heap.
That invariant is the core reason placement is in the type system.

Shared placement is not synchronization.
It does not imply atomic access, locking, actor isolation, or `Sync`.
Ordinary reads and writes of shared values use ordinary syntax, but they do not establish cross-Worker ordering, mutual exclusion, or communication.
Code that needs those guarantees uses atomics, locks, channels, actors, transactions, job systems, or another explicit protocol.
Libraries and strict profiles may require capabilities like `Send` and `Sync` for APIs that transfer or publish values, but `shared` itself is only placement.

Placement is contextual.
An aggregate field with ambient placement is interpreted in the placement of the containing value.
An aggregate field with explicit placement keeps that placement.
Therefore the compiler may lower one source aggregate into distinct concrete layouts for local and shared placement.

```ds
struct Request<T> {
    header: Header,
    body: T,
}

let local: Request<Body>;
let shared: shared Request<Body>;
```

In the local value, `header` and `body` are local.
In the shared value, the ambient `header` and `body` fields are shared.
If a field is explicitly `WithSpace<T, "local">`, the enclosing aggregate cannot be placed in shared space unless that field is some explicitly permitted cross-space handle.

Not every type can be placed in every space.
Transparent values are checked structurally, and opaque or runtime-backed values are checked by the compiler and library definitions for that space.
`shared T` is only valid when `T` can be represented in shared space.

### Capabilities

`Copy`, `Clone`, `Send`, and `Sync` are capability interfaces, not placement forms.

`Copy` means a value can be duplicated implicitly without changing ownership responsibilities.
`Clone` means code can explicitly create another value, possibly by running code or allocating.
`Send` means a value can cross a Worker boundary.
`Sync` means references to shared values can be used concurrently through the type's own API.

These capabilities are separate from placement.
`shared T` means `T` lives in shared space; it does not make `T` `Sync`.
Library APIs such as channels, Worker pools, atomics, locks, and actors can require `Send` or `Sync` when they need those stronger guarantees.

### Relations

The rules for who can point into what mostly follow from the fact that references must always be valid, and shared memory should not point into a Worker-local heap.
(And raw pointers are your own dangerous business.)

| From \ To | `T` | `shared T` | `&T` | `&shared T` | `^T` | `^shared T` | `*T` | `*shared T` |
|-----------|-----|------------|------|-------------|------|-------------|------|-------------|
| `T` | - | yes | yes | no | no | no | explicit unsafe | no |
| `shared T` | no | - | no | yes | no | no | no | explicit unsafe |
| `&T` | no | no | - | no | no | no | explicit unsafe | no |
| `&shared T` | no | no | no | -- | no | no | no | explicit unsafe |
| `^T` | no | yes | yes | no | - | no | explicit unsafe | no |
| `^shared T` | no | no | no | yes | no | - | no | explicit unsafe |
| `*T` | no | no | unsafe checked reborrow | no | no | no | - | no |
| `*shared T` | no | no | no | unsafe checked reborrow | no | no | no | - |

### Bindings

Because Destack inherits the JS/TS "Worker" model for isolation and memory ownership, module-scoped constants are owned by each _Worker_ and not actually process-global as they would be in many other languages.
For actually shared process-global globals, the binding itself can be declared as `shared`.

| Form | Binding cell | Value place | Meaning |
|----------|--------------|-------------|---------|
| `const world = new World()` | local | local | one local module binding and one local value |
| `const world: shared World = new World()` | local | shared | one local binding cell holding one shared handle |
| `shared const world: World = new World()` | shared | shared | one shared binding cell initialized in shared space |
| `shared const world: shared World = new World()` | shared | shared | same runtime meaning, explicit on both axes |

### Allocation And Destruction

The primary typed construction path is `new`.

`new` allocates heap storage, initializes a `T`, and produces the ownership form required by the destination type and the base type's affinity.

`new` is destination-typed.

The expected type decides whether construction produces a managed or owned value, with affine base types preferring owned forms in unconstrained positions.

There is no second primary typed allocation surface alongside `new`.

`raw.alloc` and `raw.free` are reserved for true raw storage only.

`drop` ends ownership of a `^T`, runs destruction, and releases owned heap storage.

`dispose` and `dispose.async` are resource cleanup protocols rather than allocation primitives.

### Borrows, Regions And Suspension

A region is one compiler-known lifetime relation for one borrow.
Explicit region spelling is only needed when a signature must relate returned borrows to input borrows.

```ds
&T                 // surface syntax
Borrowed<T, _>     // normalized form

@lifetime("a") &T
Borrowed<T, "a">
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
