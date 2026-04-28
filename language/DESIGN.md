# Destack Language Design

> **The Destack language is "TypeScript++" for building correct, optimal, integrated software systems.**
>
> This document describes the motivation and tradeoffs in choosing TypeScript and why we added what.

Destack is designed as a superset of TypeScript.
Destack also supports compiling to JS/TS targets, and works with regular JS/TS dependencies when they fit Destack's strict typed model.
So, if you don't need or want any additional features you can ignore the "++" part of Destack entirely, write completely standard `.ts` and `.tsx` files, and just stop reading right here.
For most use cases, most of the time, the "++" is happily out of sight and out of mind.

Destack adds features to TypeScript that wouldn't fit in TypeScript itself, much like `.tsx` or `.svelte` do, but for truly full stack software systems.
We support full `TSX` syntax, and modern `TS` code just works, because the Destack language ("TS++") is a superset of modern _TypeScript_.
To enable truly universal programming with TypeScript, even in high performance ("systems") use cases, we support some _additional_ stuff like manual memory management features.

Invariably, when starting with an existing language as feature rich as modern TypeScript, any _new_ additions risk becoming unpredictably combinatorial in their complexity (hello C++).
We tried hard to keep the actual net new concepts to the minimal set required to express all the missing things we needed, while also filling some gaps we experienced in the language that TypeScript cannot address directly (mostly due to its commitment to type-free emit).

## "TypeScript++"

We're very early in software.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
We cannot confidently build the next generation of software without unifying all the disparate pieces: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to a unified software foundation today that _could_ conceivably express all software (because in many ways, it already is, albeit suboptimally).
Unlike Python (and Rust and Go and insert your favorite language), the TypeScript ecosystem also has a good answer to rich frontends *and* a very strong "already runs everywhere" story because browsers are the most ubiquitous execution platform.

So, TypeScript runs everywhere, everyone knows it, and it has a massive ecosystem.
If you can compile to JS/TS, and behave like TS, you get a "new" language that doesn't actually feel new, but more like TSX or Svelte.
Then, because modern TypeScript is very close to a fully AOT-compilable language, we can build a new toolchain completely free of JS runtimes and "legacy" code while staying true to the behavior most developers already know well.

Thus, wherever Destack looks like TypeScript - e.g., `interface`, `class`, `async`/`await`, objects, templates, generics, types, everything! - it behaves like TypeScript, because it _is_ TypeScript.
Unlike with C++, our "C" - both JavaScript/TypeScript -- still work with Destack, and the `++` features are opt-in and complementary.

| Feature | What | Why |
|---------|-------------|-----|
| [**Types**](#types) | Type system extensions: primitives, nominality, tuples, generic values, associated types | Soundness, memory, precision |
| [**Declarations**](#declarations) | Declaration extensions: nominal interfaces, annotations, static `@if` | Metadata and gates |
| [**Expressions**](#expressions) | Expression extensions: blocks, patterns, trees, errors, `loop`, `using` | Better ergonomics |
| [**Operators**](#operators) | Operator interfaces, equality, indexing, overflow and runtime checks | No hidden behavior |
| [**Comptime**](#comptime) | Compile-time evaluation, generic evaluation, conditional compilation | Metaprogramming |
| [**Dispatch**](#dispatch) | Type-dependent dispatch: `extension`s, overloads, dynamic union resolution | Better ergonomics |
| [**Ownership**](#ownership) | Ownership, borrowing, local/shared spaces, and memory type algebra | Systems programming |
| [**Modules**](#modules) | Typed imports for code, data, text, and binary assets | Typed assets |
| [**Reflection**](#reflection) | Types as values, runtime type descriptors, schema validation | Metaprogramming |

## Types

Destack extends TypeScript's type system with precise primitives, nominal types ("newtypes"), ergonomic constraints, and some additional features.

### Primitives

Precise numeric types beyond TypeScript's `number`:

```ds
const id: uint64 = 12345;
const balance: float32 = 100.50;
```

Destack keeps `number` as the JS-compatible numeric supertype (aliased to `float64`).
`int` and `uint` are fixed-width aliases for `int64` and `uint64`.
`float` defaults to `float64`.
Pointer-sized integers are spelled `isize` and `usize`.

### Newtypes

TypeScript is structurally typed by default.
Destack keeps that behavior, but adds nominal forms where identity is part of the program meaning.

Newtypes are nominal wrappers that prevent mixing semantically different values:

```ds
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

const id = UserId(42);            // wraps scalar
const p = Point(1.0, 2.0);        // wraps tuple
const c = Config { debug: true };  // wraps object
```

### Structs

Structs are data-oriented value types with fixed layout without reference identity or any inheritance; they are just data with a name.
Structs may embed other structs to compose types, and structs can implement interfaces.

```ds
struct Point {
    x: float32;
    y: float32;
}
```

#### Structs vs Classes

| | struct | class |
|---|---|---|
| Reference identity | No (`===` is error) | Yes (`===` compares managed identity) |
| Inheritance | No (use embedding) | Yes (`extends`) |
| Default passing | Value | Reference |
| Default storage | Inline | Managed reference |
| JS output | Plain object | ES6 class |

Classes are reference types, so `===` compares managed object identity as usual.
Structs are value types, so `==` compares fields and `===` will just error (at compile time).

```ds
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
p1 == p2;  // true: same data

const e1 = new Entity(1);
const e2 = new Entity(1);
e1 == e2;  // false: different instances
```

#### Structs Are Nominal

Structs are nominal (like newtypes), so they must be explicitly constructed:

```ds
let x: Point = Point { x, y };  // ok
let x: Point = { x, y };        // ERROR: plain object is not Point
```

For composition, structs use embedding instead of inheritance:

```ds
struct Transform { position: Vec3; rotation: Quat; }
struct Player { ...Transform; health: int; }  // embeds Transform's fields
```

### Enums

Enums are nominal values.
Enums do not implicitly mix with their backing type, and explicit conversions are required when you want the backing value.

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

### Arrays And Tuples

Explicit tuple syntax uses parentheses:

```ds
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();
```

Arrays like `T[]` or `Array<T>` are dense, homogeneous and bounds checked by default with no holes allowed.
Thus, accessing into `T[]` just gives you a straight `T` always, because out of bounds and holes are both forbidden.
Like in JS/TS, dynamic `Array` grow automatically like you would expect.

In addition to dynamic arrays, Destack also provides fixed-size arrays with `T[N]`.
Because TypeScript already uses `T[N]` for indexed access, Destack honors that behavior when indexed access is admissible, and we have to use `N as comptime` to force fixed-size array construction in ambiguous cases.
`FixedArray<T, comptime N>` is an explicit alias for `T[N as comptime]`.

### Generic Parameters

Destack generalizes TypeScript's generic parameters.
A generic parameter can be a type parameter like in TypeScript, or a compile-time value parameter marked with `comptime`.

```ds
type Buffer<comptime N: uint> = uint8[N];

function repeat<comptime N: uint>(value: string): string {
    let result = "";
    for (let i = 0; i < N; i++) {
        result += value;
    }
    result
}
```

Type parameters still use TypeScript-style inference and constraints.
Compile-time value parameters require values known during static analysis.
Local inference can infer compile-time values from local literal arguments, such as fixed array lengths.
Inference is local to the current call and module surface; Destack does not solve exported API shapes through module cycles.

### Associated Types And Constants

Structs, classes, and interfaces can declare associated type aliases:

```ds
struct Cache<K, V> {
    type Entry = CacheEntry<K, V>;  // associated type
    
    entries: Entry[],
}
```

Associated types are resolved at compile time and can reference generic parameters.
Associated projection uses the same member lookup and substitution model as value members.

Associated types are static members of a type owner.
Projection substitutes the owner's generic arguments first, then resolves the associated member.

```ds
type Entry = Cache<string, User>.Entry;
```

Interfaces may declare abstract associated types and defaults.
Implementors must provide concrete definitions for abstract associated types, and may override defaults when the replacement satisfies the declared constraint.

Associated types can have their own generic parameters.
Generic associated types cover cases where the associated shape depends on both the owner and a later type or compile-time value.
Generic associated types support the same generic parameter forms as ordinary declarations, including type parameters and `comptime` value parameters.

```ds
interface Slice<T> {
    type View<U>;
}

struct Buffer<T> {
    type View<U> = BufferView<T, U>;
}
```

Class-shaped declarations can also declare associated compile-time constants with `comptime const`.
Associated constants are static members, have no instance storage, and must be statically evaluable.
When an associated constant is used in a type expression, its evaluated value participates in that resulting type.
Associated constants do not otherwise become part of a type's identity merely by existing.

```ds
interface LogStore<Record> {
    comptime const SegmentRows: uint = 1024;
    type Segment = Record[this.SegmentRows];
}
```

### Constraints

`where` clauses for readable generic constraints:

```ds
function merge<T: int, U>(): T where (
    U: Comparable<T>
) { }
```

### The `this` Type

Destack supports TypeScript's polymorphic `this` type for instance members, and also allows it in static type positions.
`this` is type-only and resolves to the surrounding receiver or containing type.

## Declarations

### Interfaces And Classes

Interfaces and classes behave like TypeScript unless Destack explicitly adds something.
Interfaces are structural by default.
Classes are reference types with identity, constructors, static members, methods, inheritance, and ordinary TS-shaped method lookup.

Destack does not use TypeScript's prototype dynamism as a language extension point.
Extensions, nominal interfaces, and static metadata are the typed extension surfaces instead.

### Nominal Interfaces

TypeScript interfaces are structural, i.e., any type with matching shape satisfies the interface.
This is usually what we want, but sometimes nominality is required for a contract, and in those cases the TS ecosystem usually uses branding symbols.

Destack adds real **nominal interfaces** using the `newtype` modifier on `interface` declarations:

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

Nominal interfaces require **explicit `implements`** declarations.
Structural compatibility alone doesn't satisfy the constraint.
Nominal interfaces are used for operator interfaces like `Add` and `Compare`, and for capability traits like `Send`, `Sync`, `Copy`, and `Clone`.

The `newtype` modifier on `interface` follows the same pattern as `newtype` on type aliases, making a `newtype interface` more like a nominal trait in other languages.

```ds
// structural interface: requirements only
interface Drawable {
    draw(): void;
}

// nominal interface: defaults allowed
newtype interface Print<T> {
    print() {
        // do nothing by default
    }
}
```

### Annotations And Decorators

Destack extends decorators (`@`) to work on many more language constructs than TypeScript: declarations, statements, members, parameters, types, match arms, and more.
TypeScript decorators are a compatible subset.
Destack annotations are metadata and transform hooks the compiler can see.

```ds
@deprecated("use newAPI instead")
function oldAPI() { }

@memoize
function expensive() { }

@unroll
for (let i = 0; i < 4; i++) { }

// on struct members
struct User {
    @validate(minLength(1))
    name: string,
}

// on function parameters
function process(@nonempty input: string) { }

// on reference types
function kernel(data: @space("shared") &Point) { }

// on match arms
match (result) {
    @cold
    Err(e) => handleError(e),
    Ok(v) => v,
}
```

Decorator behavior depends on what the decorator resolves to:
- **Function**: Transforms the target, `@foo body` desugars to `foo(body)`
- **Newtype**: Compile-time metadata, available for reflection but stripped from output

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
Multiple `@if` annotations combine with logical AND.

## Expressions

### Blocks And Conditionals

In TypeScript, control flow expressions like `if` are a statement, and you need a ternary or temporary to get a value out.
To enable more ergonomic data flow and particularly better pattern matching capabilities, Destack also supports statements as expressions ("everything is an expression").
Like in similar languages, the last non-statement expression (no trailing `;`) becomes the value of the expression.

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

Modern `match` with full pattern matching and exhaustiveness checking:

```ds
match (result /* Result<T, E> */) {
    { kind: 'ok', value } => process(value)
    { kind: 'err', error } if (isRetryable(error)) => retry()
    { kind: 'err', error } => fail(error)
}
```

As you would expect, the type of a match expression is the union of its case body types.

Patterns can appear in `match`, `if let`, `let ... else`, and destructuring bindings.
The core pattern families are wildcard, binding, literal, tuple, object/struct, array/slice, variant/newtype, union, and guarded patterns.

```ds
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at ${x}, ${y}`
}
```

#### Fallibility

Some patterns are irrefutable, meaning they always match.
Plain binding patterns, `_`, and exact destructuring of already-known tuple shapes are irrefutable.

Other patterns are refutable, meaning the match can fail.
Variant patterns, literal patterns, guarded patterns, nullable unwrapping, and most structural tests are refutable.

Refutable patterns require syntax that says what happens on failure: a `match` fallback arm, an `else` branch for `if let`, or an `else` continuation for `let ... else`.

```ds
let Some(value) = maybe else {
    return Result.err("missing value");
};
```

#### Exhaustiveness

`match` exhaustiveness is enforced when the compiler can prove the value set is finite.
This includes enums, literal unions, discriminated unions, fixed-size tuples, and other fully-known finite shapes.

Irrefutable fallback arms like `_` satisfy exhaustiveness.
When any arm has a guard, or when the compiler cannot prove the input is finite, a fallback arm is required.

### Loops

Infinite loops with `loop`.
All TypeScript loop forms still work.
`loop` is the explicit infinite loop form, and can produce a value through `break value`.

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

`using` is explicit scoped cleanup scheduling with TS-shaped surface syntax.
Resources are cleaned up at lexical scope exit in LIFO order, and `await using` runs async cleanup when required.
The same cleanup capabilities also power ownership based destruction.
Affine owned values may therefore be cleaned up earlier, at their last proven use, even without `using`.
`using` does not replace ownership.
It chooses one cleanup scope explicitly.

```ds
using file = openFile(path);
await using conn = openConnection();
```

Cleanup runs when the scope exits for any reason: fallthrough, `return`, `break`, `continue`, `throw`, or `?`.
`using` requires `Disposable | null | undefined`.
`await using` requires `AsyncDisposable | Disposable | null | undefined`.
In loop initializers, a `using` resource is per-iteration and is disposed at the end of that iteration.
At module top level, a `using` resource is disposed when module evaluation completes.

Ownership and `using` are intentionally separate mechanisms.
`^T` controls memory ownership and lifetime.
`using` controls resource protocol disposal and pins cleanup to a lexical scope.

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

### Errors

Destack strongly encourages **Result-first error handling** inspired by Rust: recoverable errors use `Result<T, E>`, while exceptions exist for compatibility, interop, and migration.
`Result<T, E>` with `?` and `??` remains the preferred everyday style.

#### Result Types

The standard library provides `Result<T, E>` as the primary error handling mechanism:

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

#### Try Protocol

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
- Use `?` or `??` inside the block to propagate `Try` errors into the catch
- When a `?` is inside a `try` with a catch, `Try.fromError` is not required

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

## Operators

Destack keeps TypeScript's operators where they already have clear JavaScript semantics, and adds typed overloads only where the operator maps cleanly to an explicit protocol.
Operator overloading is receiver-based and nominal: a type must explicitly implement the corresponding nominal operator interface.

### Operator Dispatch

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

Structural compatibility is not enough to overload an operator.
That keeps accidental method names from changing expression meaning.

### Equality And Identity

`==` is value equality and can use `Equal`.
`===` is identity equality and is not overloadable.

Classes have identity, so `===` compares managed object identity.
Structs do not have reference identity, so `===` on structs is a compile-time error.
Use `==` for value comparison.

### Arithmetic And Checks

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

Division, remainder, shifts, bounds checks, null checks, and other runtime checks are controlled by profile options.
The point is to make unsafe behavior explicit instead of letting it leak in through target defaults.

### Indexing

Array and tuple indexing is bounds checked and returns the element type directly.
`noUncheckedIndexedAccess` still matters for index signatures and other dynamic indexers, but dense arrays do not produce `T | undefined` on every access.

Custom indexing can be modeled through nominal interfaces, but builtin arrays, tuples, slices, tensors, and string-like types keep their builtin semantics.

### Special Operators

Some operators are deliberately not overloadable because their control-flow or type-system behavior is too fundamental.
This includes `&&`, `||`, `?.`, `as`, `satisfies`, `typeof`, `keyof`, `extends`, `implements`, `is`, `instanceof`, `?`, and `??`.

`?` and `??` are protocol-driven, but they are not ordinary overloadable operators.
Their semantics are fixed by the language and implemented through `Try`.

## Comptime

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
The `comptime` keyword requires that an expression must be evaluated at compile time (otherwise it is a compile error):

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
    if (n <= 1) { 1 } else { n * factorial(n - 1) }
}

const FACT_10 = comptime factorial(10);    // compile time
const dynamicValue = factorial(getUserInput()); // runtime (in this case, at module initialization time)
```

Functions are not marked explicitly as either "comptime" or "runtime" functions, instead, the call site determines when a function runs.

### Execution Model

Destack distinguishes **static execution** and **comptime execution**:

- **Static execution** is a small, closed-form subset that can be evaluated during Analyze.
  This includes literals, arithmetic on literals, known constants, generic value arguments, associated constants, and other syntax that can be folded without executing arbitrary user code.
  These are required for generic value parameters and type-level arguments.
- **Comptime execution** evaluates `comptime` expressions and blocks by running lowered MIR in the VM interpreter during Execute.
  Results are written back into the program as constants and dead branches are eliminated.

Static execution must not depend on full comptime execution.
This avoids dependency cycles between generic solving, type normalization, and runtime-shaped comptime code.
Full comptime evaluation happens after monomorphization and lowering, with full type information available.

### Generic Evaluation

Generic parameters, associated types, associated constants, conditional types, mapped types, and ownership algebra are all resolved by static evaluation.
Generic type parameters range over types.
Generic `comptime` parameters range over statically known values.

Associated projections normalize by substituting the owner's generic arguments first:

```ds
type Row = Matrix<4, 4>.Row;
```

Associated constants must be statically evaluable.
Conditional and mapped types run over normalized type algebra.
Ownership and placement are part of that algebra through `Form<T, O, S, R>`, so `BaseOf`, `OwnershipOf`, `SpaceOf`, and `RegionOf` are ordinary type-level queries.
`SupportsSpace<T, S>` is the corresponding intrinsic type-level placement predicate.

Inference is local.
The compiler infers generic arguments at the call site and inside the current module, but exported APIs must expose enough explicit shape for importing modules.
Destack does not solve API types through module cycles.

Comptime conditions enable branch elimination and, for type relations like `T extends U`, type narrowing:

```ds
function process<T, Context: CacheContext<T>>(ctx: Context, key: T) {
    if (comptime Context extends EvictableContext<T>) {
        ctx.onEvict(key);  // context is narrowed; branch eliminated if not satisfied
    }
}
```

Both branches of a comptime condition must type check unless the branch is removed by static `@if`.
That keeps ordinary generic code stable under different profiles while still allowing profile-specific declarations when needed.

### Comptime Blocks

Comptime blocks can also appear as struct/class members for compile-time assertions:

```ds
struct Buffer<comptime size: uint> {
    comptime {
        assert(size > 0 && size <= 65536);
    }
    data: uint8[size],
}
```

Member comptime blocks run once per type instantiation.
Module-level comptime blocks run during module compilation.

### Compile-time Eval

At comptime, `eval` and `new Function` mean compile-time code evaluation, not runtime string execution.
The source string must be statically known.
The code is parsed as Destack, type checked in the current module context, evaluated in the comptime VM, and cached like other comptime work.

```ds
const add = comptime new Function("a", "b", "return a + b") as (a: int, b: int) => int;
const value = comptime eval("add(1, 2)");
```

Runtime `eval` and `new Function` are JS compatibility features, and portable Destack code should not depend on them.

## Dispatch

TypeScript has parametric polymorphism ("generics") but does not support type-based dispatch (by design).
Destack adds type extensions and real overloading for type-based dispatch and operator overloading.

### Extensions

Destack introduces extensions to add methods and static constants for any _nominal_ type:

```ds
extension of Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extensions require **nominal types** with identity.
This includes `struct`, `class`, `enum`, `newtype`, and primitive types declared in the prelude (`int32`, `string`, etc.).
Type aliases (`type X = ...`) and inline structural types (`{ x: number }`) cannot be extended (because that would be very unpredictable).

To extend a structural shape, wrap it in a nominal type:

```ds
type Point = { x: number, y: number };

extension of Point { ... }  // ERROR

newtype Point = { x: number, y: number };
// works - extend newtype / struct / class / ..
extension of Point { ... }  // ok
```

Extension visibility is basically as you would expect:
- **Same file as type**: Extensions are automatically visible wherever the type is used.
- **Anonymous on foreign type**: Only visible in the file where declared (`extension of int32 { ... }`).
- **Named on foreign type**: Must be explicitly imported to use (`export extension DateUtils of Date { ... }`).

### Overload Resolution

Real function and method overloading with distinct implementations:

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }
```

Following TS, to avoid ambiguity, Destack uses **declaration order**, i.e., the first matching overload wins.
Applicability includes generic argument inference and validation, including `comptime` generic value parameters.
Overload order is defined at the declaring module and is forwarded unchanged across exports, reexports, and namespace imports.

The compiler should warn when an earlier overload shadows a later one completely.

### Dynamic Resolution

When the receiver of a member access or method call is a union, Destack resolves the member for each union variant.
If all variants resolve to the same symbol, the call is static.
If the symbols differ, the compiler records a dynamic resolution and reifies it into `if (receiver is Type)` branches.

Dynamic resolution only applies when every union variant exposes the member.
Arguments must satisfy all candidate signatures, and the resulting type is the union of per-candidate return types after substitutions.
Extension methods participate in member resolution, too.

## Ownership

TypeScript does not encode "ownership" in its type system: all reference types are implicitly GC managed, and all value types are copied by default.
This is convenient, but sometimes we want to take direct ownership of memory, whether for better control and performance, or just to express and enforce invariants in the code.
Destack adds explicit, optional modifiers for controlling memory ownership and placement inspired by Rust and Mojo's ownership models with `^T` as the "owned" signifier.

Further, as TS already has a strong notion of local memory as the implicit memory model, we also support explicit shared memory model as part of a generalized, explicit notion of "place" local to a worker, shared across workers, or another address space.
On the shared heap side, this is basically a generalization around `SharedArrayBuffer`-like semantics.
As with ownership, most of the time, developers don't need to think about placement, but it is very useful in certain situations.

### Forms

| Form | Ownership | Region | Place | Liveness | MIR shape | Value |
|------|-----------|--------|-------|---------------|-----------|-------|
| `T` | managed | none | ambient | keeps the referent alive | `ref<T, managed, space(local)>` | managed heap handle |
| `shared T` | managed | none | shared | keeps the referent alive | `ref<T, managed, space(shared)>` | managed shared handle |
| `&T` | borrowed | inferred or explicit | ambient | requires liveness | `ref<T, borrowed, space(X)>` | semantic borrow or projection |
| `&shared T` | borrowed | inferred or explicit | shared | requires liveness | `ref<T, borrowed, space(shared)>` | semantic shared borrow or projection |
| `^T` | owned | none | ambient | owns the referent | `ref<T, owned, space(X)>` | owned heap handle |
| `^shared T` | owned | none | shared | owns the referent | `ref<T, owned, space(shared)>` | owned shared heap handle |
| `*T` | raw | none | ambient | does not keep anything alive | `ref<T, raw, space(X)>` | unsafe raw typed pointer |
| `*shared T` | raw | none | shared | does not keep anything alive | `ref<T, raw, space(shared)>` | unsafe shared raw typed pointer |

Managed and owned values both live on the heap.

Heap storage is traced whenever `T` may contain references.

Raw storage is never traced.

### Local And Shared Space

Local space is the default per-worker or per-isolate memory space.
Ordinary managed objects, arrays, strings, functions, closures, and module bindings live in local space unless a type or binding says otherwise.

Shared space is runtime-shared memory visible across workers or isolates.
It is the typed, generalized version of the `SharedArrayBuffer` idea rather than a separate language.
`shared T` means `T` re-based into shared space, i.e. `WithSpace<T, "shared">`.
`local` is not a surface keyword.
The local space can be named explicitly through `WithSpace<T, "local">` or `@space("local")` where an explicit space annotation is needed.

Local values may point to shared values.
Shared values must not point into local memory.
That invariant is the core reason placement is in the type system.

Shared placement is not synchronization.
Cross-worker mutation still needs explicit atomics, locks, channels, or another library protocol.

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

`SupportsSpace<T, S>` is the intrinsic type-level placement predicate.
It is structural for transparent values and compiler-defined for opaque or runtime-backed values.
`shared T` requires `SupportsSpace<T, "shared">`.

### Relations

The rules for who can point into what mostly follow from the fact that references must always be valid, and shared memory should not point into local memory.
(And raw pointers are your own dangerous business.)

| From \ To | `T` | `shared T` | `&T` | `&shared T` | `^T` | `^shared T` | `*T` | `*shared T` |
|-----------|-----|------------|------|-------------|------|-------------|------|-------------|
| `T` | - | no | yes | no | no | no | explicit unsafe | no |
| `shared T` | no | - | no | yes | no | no | no | explicit unsafe |
| `&T` | no | no | - | no | no | no | explicit unsafe | no |
| `&shared T` | no | no | no | -- | no | no | no | explicit unsafe |
| `^T` | no | no | yes | no | - | no | explicit unsafe | no |
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

Declarations that store borrowed fields are implicitly region generic.
Owned fields make the enclosing type affine.
Affine storage-bearing positions default to owned storage rather than implicit heap storage.
Borrowed fields make the enclosing type region generic.

Managed objects may contain owned fields.

That integration is required so ownership composes with ordinary managed programming rather than creating a second disjoint language.

Types containing owned fields are affine and therefore do not silently copy.

Ordinary borrows should not cross `await`, suspension, or worker transfer boundaries at first.
Owned values may cross suspension points by moving into the coroutine frame.
If an owned value is dead before suspension, cleanup is inserted before the suspend edge.

Moving an owned value invalidates the previous binding and every borrow rooted in it.
Dropping or freeing a value invalidates every borrow rooted in that value.
Borrow checking is provenance-based: a borrow tracks the owner or storage root it depends on, and derived borrows preserve that root.

### Type Algebra

Internally, Destack normalizes ownership and place into one type-addressable `Form<T, O, S, R>`.
That lets ordinary TS-style type algebra talk about base type, ownership, place, and borrow region directly, which makes for some very convenient conditional and mapped type algebra.

Traceability is derived from the base type and layout metadata rather than from ownership itself.

| Surface spelling | Algebraic spelling |
|------------------|--------------------|
| `T` | `Managed<T>` in storage bearing positions |
| `&T` | `Borrowed<T, _>` |
| `^T` | `Owned<T>` |
| `*T` | `Raw<T>` |
| `shared T` | `Shared<T>` |
| `@space("shared") T` | `WithSpace<T, "shared">` |
| `SupportsSpace<T, "shared">` | placement validity query |

The core kernel is:

```ds
type Form<T, O = "managed", S = "local", R = never> = ...

type BaseOf<T> = ...
type OwnershipOf<T> = ...
type SpaceOf<T> = ...
type RegionOf<T> = ...

type OwnershipOr<T, D> = ...
type SpaceOr<T, D> = ...
type SupportsSpace<T, S> = ...

type Managed<T> = ...
type Borrowed<T, R> = ...
type Owned<T> = ...
type Raw<T> = ...
type Shared<T> = ...

type WithBase<Q, T> = ...
type WithOwnership<Q, O> = ...
type WithSpace<Q, S> = ...
type WithRegion<Q, R> = ...
```

## Modules

Destack supports importing various file types beyond code modules, following Bun's approach to asset imports.
Ordinary JavaScript and TypeScript module syntax works as you would expect.

### Import Meta

`import.meta` exposes module and profile metadata during static and comptime evaluation.
The values are fixed for the active profile and are not runtime dependent.

```ds
const emit = import.meta.emit;
emit satisfies "js" | "ts" | "html" | "wasm" | "native";
```

`native` means CPU/OS ABI output, such as an executable, object, or library for a concrete platform target.
`wasm` remains its own emit format even when the runtime executes wasm directly.

The important profile fields are:

| Field | Meaning |
|-------|---------|
| `import.meta.url` | current module URL |
| `import.meta.path` | current local file path, when available |
| `import.meta.dir` | current local directory, when available |
| `import.meta.emit` | output artifact format |
| `import.meta.target` | target platform and ABI |
| `import.meta.runtime` | runtime environment |
| `import.meta.debug` | debug/development build flag |
| `import.meta.test` | test build flag |
| `import.meta.env` | configured build environment |

### Inference Boundaries

Destack inference is local.
Within a module, inference can use local declarations and local expression context.
Across modules, exported APIs must expose enough explicit type information for importing modules to analyze them without solving a module graph cycle.

That keeps module analysis local and makes public surfaces explicit.

### Data Modules (JSON, TOML, YAML)

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
(The same file with different loaders produces different modules, of course.)

## Reflection

In TypeScript, types are - by design - erased at runtime.
This was critical for early adoption, but it also means you can't easily perform runtime type checks or any meaningful reflection (without additional libraries or build steps).
Destack supports `Type` as a first-class value so reflection has one typed path instead of ad hoc schema side channels.

### Type Descriptors

Every nominal type `T` in Destack has a corresponding descriptor value of type `Type<T>`:

```ds
struct User {
    name: string;
    age: uint;
}

// User in type position: the type
let u: User = User { name: "Alice", age: 30 };

// User in value position: the type descriptor
const UserType = User;              // UserType: Type<User>
UserType.name                       // "User"
UserType.fields                     // [{ name: "name", type: string }, ...]
```

For classes and structs, the constructor value can also serve as the descriptor.
`typeOf(value)` returns a descriptor for the value's static type, unlike JavaScript's runtime `typeof`, which returns a coarse string.
The type-level `typeof` operator still means TypeScript's value-type query.

### Decorator Metadata

Decorator information is accessible at runtime:

```ds
@deprecated("use newAPI")
function oldAPI() { }

oldAPI.decorators       // [{ name: "deprecated", arguments: ["use newAPI"] }]
```

### Profile

Reflection is useful for schema validation, serialization, metadata, tooling, and runtime type checks.
It also has output-size and portability costs.

Profiles may control how much reflection metadata is emitted.
The type system should still be able to reason about `Type<T>` even when a target strips runtime metadata that code does not use.

Reflection metadata is separate from dispatch metadata.
`TypeId` is compact runtime type identity.
`Type<T>` is a descriptor used for reflection.
Class vtables and nominal interface implementation tables are dispatch artifacts.
They may share type identity, but ordinary dispatch must not require rich reflection metadata.

## Compatibility

### Modern Strict TypeScript

**Destack aims for 100% compatibility with _modern_ TypeScript.**
To be completely fair, this is a little sneaky, because we get to decide what "modern" means - but really, it just means that much of the deprecated TS legacy stuff is unsupported, and most _runtime dynamic_ JS features are profile-gated or deliberately out of scope (`Function`, `eval`, `prototype` modification, etc.).

### `.ds` Syntax Differences

Syntax-wise, for `.ds` files, a few obscure syntax patterns work differently due to built-in TSX support and additional typing features:

| Pattern | `.ts` | `.tsx` | `.ds` |
|---------|-------|--------|-------|
| `<T>() => ...` | Generic arrow | Ambiguous (use `<T,>`) | Ambiguous (use `<T,>`) |
| `(a, b, c)` | Comma operator | Comma operator | Tuple literal |

Fortunately, these patterns already rarely appear in production code:
- The **generic arrow** ambiguity already exists in `.tsx` files, and `.ds` inherits this since it supports TSX syntax natively.
  The workaround (`<T,>`) is standard practice in TSX codebases.
- The **comma operator** is mostly seen in minified code or obscure one-liners. Destack uses `()` for tuples instead, which is more explicit and composes better with the type system than TypeScript's `[T, U]` array syntax.

### Profile Gates

Profiles control how strict and portable a build must be.
The important gates are:

| Option | Meaning |
|--------|---------|
| `noExceptions` | disables `throw`, while keeping `try`/`catch` for `Try` propagation |
| `noDynamicEvaluation` | disables runtime `eval` and runtime `new Function` |
| `noDynamicShapes` | disables runtime shape generation required by declaration expressions on portable targets |
| `noImplicitManaged` | requires explicit ownership forms where managed defaults would otherwise appear |
| `noManaged` | forbids managed allocation for profiles that need explicit memory only |

Safety profiles also control overflow checks, bounds checks, null checks, division checks, shift checks, and the failure mode for those checks.

### Unsupported Legacy And Dynamic JavaScript

Destack does not and will not support:
- **Flow**: We support TypeScript only.
- **Sloppy mode**: Destack targets modern strict-mode JavaScript/TypeScript. Non-strict ("sloppy mode") behaviors like duplicate function declarations or `yield` as an identifier are not supported. This aligns with how TypeScript modules work (always strict) and modern best practices.
- **Declaration expressions (native targets)**: Declaration expressions like `const C = class { }` require runtime type generation, which is incompatible with ahead-of-time compilation. Use named declarations instead. On JS targets, enable `noDynamicShapes` for portability.
- **XML namespace resolution**: Destack does not implement XML `xmlns` namespace binding semantics.
  Namespaced tree tags like `<svg:path />` are treated as intrinsic string tag names (`"svg:path"`).
