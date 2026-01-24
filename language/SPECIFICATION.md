# Destack Language Specification

<!-- TODO #Cleanup: dedupe DESIGN / SPECIFICATION -->

The Destack language is "TypeScript++" for building optimal, correct, integrated full-stack software systems.
This document describes the syntax and semantics of **`.ds` files**.
`.ts` and `.js` files work the same as before (for modern strict mode, with minor caveats depending in options and targets).

## Literals

Destack supports all JavaScript/TypeScript literals with some additions.

### Numeric Literals

Numeric literals work just like in JavaScript / TypeScript:

```
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

```
"hello"              // double-quoted string
'a'                  // character (single Unicode codepoint)
```

### Template Literals

Template literals support interpolation and can use tags, just like in JavaScript / TypeScript:

```
`hello`                           // simple template
`hello ${name}`                   // interpolated template
`${a} + ${b} = ${a + b}`          // multiple interpolations
sql`SELECT * FROM users`          // tagged template
html`<div>${content}</div>`       // another tagged template
```

Tagged templates call a function with the string parts and interpolated values, enabling DSLs for SQL, HTML, CSS, GraphQL, and more.

### Regex Literals

Regex literals work exactly like JavaScript/TypeScript:

```
/pattern/            // regex
/\d+/g               // regex with flags
/hello\s+world/i     // case insensitive
```

### Collection Literals

Array and object literals work like TypeScript:

```
[1, 2, 3];             // array
{ a: 1, b: 2 };        // anonymous object literal
```

Destack adds tuple and typed struct literals:

```
(1, 2, 3);             // tuple
Point { x: 1, y: 2 };  // typed struct literal
```

### Range Literals

Destack supports range literals for iteration and slicing:

```
1..10                // exclusive range [1, 10)
1..=10               // inclusive range [1, 10]
start..end           // variable ranges
```

### Tree Literals

Tree literals use TSX syntax for hierarchical data structures.
Destack is fully TSX-compatible: copy-paste from `.tsx` files just works.
Unlike TSX which is specific to React, Destack's tree literals work with any tree-shaped data:

```
<Entity id={1}>
    <Child name="foo" />
    <Child name="bar" />
</Entity>

<Prompt>
    <System>You are a helpful assistant.</System>
    <User>{userMessage}</User>
</Prompt>

<Level difficulty={3}>
    <Player position={spawn} />
    {enemies.map(e => <Enemy {...e} />)}
</Level>
```

Expression containers `{expr}` inside tree literals follow TSX semantics: they contain a single expression.
For multi-statement blocks, use `do { }`:

```
<Component
    simple={computeValue()}
    complex={do { let x = prepare(); transform(x) }}
/>
```

The tree literal syntax is customizable via traits, so your domain types can define how they're constructed from tree syntax.

## Types

Destack extends TypeScript's type system with precise primitives.
It adds explicit reference semantics and types as values.
It supports static parameterisation of values and other advanced type features.

### Inference

Destack prefers explicit types at public boundaries.
Public boundaries include exported declarations and public members.
Exported bindings may use local surface inference when their type is determined from local syntax
without relying on _inferred_ types from other modules.
Inference is local and never relies on whole program analysis.

#### Explicit Typing

Most non local constructs must be explicitly typed unless surface inference applies.

- Non lambda functions, methods, and constructors must annotate dynamic parameters.

```
export function sum(left: int32, right: int32): int32 {
    left + right
}
```

- Non lambda functions, methods, and constructors must annotate return types unless surface
  inference applies for an export.

```
export function version(): string {
    "v1"
}
```

- Public fields and properties must declare types.

```
export struct User {
    name: string
    age: uint32
}
```

- Function types in type declarations must annotate parameters and return types.

#### Public boundaries and surface inference

Exported bindings may omit explicit annotations when their types can be inferred from local syntax,
including inferred return types and locally resolved references.
Other modules consume the published export types without re-inferring the defining module.
Inference cycles across modules are forbidden: if an exported binding participates in a cycle,
it must be explicitly annotated to break the cycle.

Examples:

```
export const version = "v1";        // ok, inferred from local literal
export let counter = 0;             // ok, inferred and widened locally
export function add(a: int, b: int) { // ok, return type inferred locally
    a + b
}
export const shared = other.value;  // ok when no inference cycle is formed
```

```
export interface Parser {
    parse(input: string): uint32
}
```

#### Implicit Typing

Inference is allowed when the surface is local or contextual.

- Static parameters may specify types but are not required.

```
function identity<T>(value: T): T {
    value
}

function compute<Flag: boolean>(data: uint8[]): uint8[] {
    data
}
```

- Lambdas may omit parameter and return types when a contextual type is available.

```
const handler: (value: string) => uint32 = (value) => value.length;
```

- Local bindings may infer types from their initializer.

```
let count = 0;
const label = "ready";
```

- Object literal fields may omit annotations when the binding is typed or uses `satisfies`.

```
const options: Options = { retries: 3, verbose: false };
const settings = { retries: 3, verbose: false } satisfies Settings;
```

### Primitives

TypeScript has `number`, `string`, `boolean`, `bigint`, `symbol`, `null`, `undefined`, and `void`.
Destack adds precise numeric types while keeping the originals as aliases.

#### Special Types

All special types work like in TypeScript, with Destack extending the receiver-aware `this` type.

- `void` - empty type (no value)
- `null` - explicit zero/unset value
- `undefined` - uninitialized value
- `never` - bottom type (unreachable)
- `any` - top type (discouraged, forbidden in strict mode)
- `unknown` - explicit top type
- `this` - receiver type (see below)

#### The `this` Type

Destack supports TypeScript's polymorphic `this` type for instance members, and extends it to static type positions.
`this` is only valid in type positions and resolves based on the surrounding declaration:

- In instance members, `this` resolves to the concrete receiver type.
- In static type positions (like static members or static arguments), `this` resolves to the containing type itself.

#### Booleans

Booleans work unchanged.

#### Numerics

TypeScript uses `number` for all numerics, which Destack aliases to `float64`.

Destack further adds more precise integer types:
- `int8`, `int16`, `int32`, `int64`, `int128` (signed)
- `uint8`, `uint16`, `uint32`, `uint64`, `uint128` (unsigned)
- `isize`, `usize` - pointer-sized integers
- `int` / `uint` - default integer width for the current compiler configuration (default 32-bit)
- Arbitrary width integers: `int3`, `uint17`, etc.

Destack also supports specifying `float` explicitly:
- `float32`, `float64`, and arbitrary widths like `float16` and `float128`
- `float` - default float width for the current compiler configuration (default 64-bit)
- `number` - JS-compatible numeric supertype that accepts precise ints/floats

Unlike TypeScript's single `number` type, Destack's precise integers behave like real machine integers:
they have defined overflow semantics (wrapping, saturating, or trapping), proper bitwise operations.
(Of course, when transpiled to )

Assignments from `number` to a precise numeric type (`int32`, `float64`, and so on) require an explicit conversion.

#### Characters and Strings

In addition `string`, Destack supports a single `character`:
- `string` - UTF-8 string (same as TypeScript)
- `character` - single Unicode codepoint

Character and string are distinct types.
No implicit widening between them is performed.

#### Template Literal Types

Template literal types use the same backtick syntax in type positions.
Spans inside `${...}` are type expressions that must be stringifiable.

Stringifiable span types include:
- `string`, `number`, `bigint`, `boolean`, `null`, `undefined`, `any`
- Template literal types and unions of stringifiable types

Assignability rules:
- A string literal is assignable to a template literal type when it matches the literal parts and each span constraint.
- A template literal type is assignable to `string`.
- Spans of `never` accept no strings, so template literals containing `never` are uninhabited.

Literal span matching:
- `${null}` only matches `"null"`.
- `${undefined}` only matches `"undefined"`.
- `symbol` is not stringifiable and cannot appear in template literal spans.

Conditional inference:
- Template literal types participate in conditional `infer` by splitting the string left to right on literal parts.
- Template literal `infer` only matches when the checked type is a string literal or template literal type.
- `string` and `unknown` do not match template literal patterns and fall back to the else branch.
- `any` yields the union of both branches, matching TypeScript's conditional behavior.

Numeric span matching:
- `number` spans match TypeScript numeric strings: decimal forms with optional sign, fractional part, and exponent, plus hex/binary/octal literals without a sign.
- `NaN` and `Infinity` are not matched by `${number}`.
- Whitespace is not allowed in numeric string forms.
- `bigint` spans match bigint literal strings with optional leading `-` and optional hex/binary/octal prefix; whitespace and `+` are not allowed.
- TS++ numeric primitives (`int32`, `uint64`, `float32`, and so on) use the same matching rules with the appropriate range checks and integer enforcement.
- When `infer` binds numeric spans, only canonical numeric strings infer literal types; non-canonical numeric strings infer the primitive type instead.

### Type Aliases and Newtypes

Type aliases work like TypeScript, with Destack adding `newtype` for nominal (distinct) types:

```
type Point = { x: float32, y: float32 };   // structural (TypeScript)
newtype UserId = int64;                     // nominal (distinct type)
```

A `newtype` creates a distinct type—`UserId` and `OrderId` won't mix even if both are `int64`.
Newtypes are nominal and share the runtime representation of their wrapped type.
Newtypes can wrap scalars, tuples, or objects:

```
newtype UserId = int64;                    // wraps scalar
newtype Point = (float32, float32);        // wraps tuple
newtype Config = { debug: boolean };       // wraps object
```

Construction syntax matches the underlying type:

```
const id = UserId(42);                     // scalar: Name(value)
const p = Point(1.0, 2.0);                 // tuple: Name(elements...)
const c = Config { debug: true };          // struct: Name { fields... }
```

Pattern matching also works with newtype constructors.
Patterns must use the newtype name as the tag.
The inner pattern matches the underlying representation.
Object newtypes use tagged object patterns (for example, `Config { debug }`).
Untagged object patterns do not match newtypes.

```
match (id) {
    UserId(0) => "system"
    UserId(n) => `user ${n}`
}
```

Newtypes can have methods via extensions (see Extensions below).
The same mechanism works for all types: structs, enums, newtypes, even foreign types and builtin primitives like `int32` or `Date`.

### Unions and Intersections

Structural combinator types work like in TypeScript:

```
int32 | string | null      // union
A & B                      // intersection
```

#### Discriminated Unions

Destack supports TypeScript-style discriminated unions directly:

```
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

// or use the newtype Result
function divide(a: int, b: int): Result<int, string> {
    if (b == 0) {
        Result.err("division by zero")
    } else {
        Result.ok(a / b)
    }
}

const result = divide(10, 2);
if (result.kind == 'ok') {
    print(`result: ${result.value}`);
} else {
    print(`error: ${result.error}`);
}
```

This is standard TypeScript and works unchanged in Destack.

#### Discriminant Tag Interning

Discriminant literals are interned to integer tags at compile time for native targets.
Supported literal kinds:
- string
- number
- bigint
- boolean
- null
- undefined
- unique symbol

NaN is not a valid discriminant literal.
-0 and 0 are treated as the same discriminant literal.
Tags are assigned deterministically within a compilation.
Each discriminated union emits a tag value table with literal values in tag order.
Reading a discriminant field loads the literal value from that table.
Duplicate discriminant values are a type error.
Tag values are not stable across different compilations or compiler versions.
Code should never serialize or persist tag integers.

#### Result Types

For richer `Result` types with methods, use nominal structs and newtypes:

```
struct Ok<T> { kind: 'ok' = 'ok', value: T }
struct Err<E> { kind: 'err' = 'err', error: E }
newtype Result<T, E> = Ok<T> | Err<E>
```

Since `Result` is a nominal type, it can be extended with methods:

```
extension<T, E> for Result<T, E> {
    static ok(value: T): Result<T, E> { Result(Ok { value }) }
    static err(error: E): Result<T, E> { Result(Err { error }) }

    map<U>(f: (T) => U): Result<U, E> {
        match (this) {
            Ok { value } => Result.ok(f(value))
            Err _ => this
        }
    }

    unwrap(): T {
        match (this) {
            Ok { value } => value
            Err { error } => throw error
        }
    }
}
```

Usage:

```
function divide(a: int, b: int): Result<int, string> {
    if (b == 0) {
        Result.err("division by zero")
    } else {
        Result.ok(a / b)
    }
}

// Method chaining
divide(10, 2).map(x => x * 2).unwrap()

// Pattern matching (newtype unwraps automatically)
match (divide(10, 2)) {
    Ok { value } => print(`result: ${value}`)
    Err { error } => print(`error: ${error}`)
}
```

Both approaches are compatible.
Newtypes share the underlying runtime representation, so you can still use structural matching like `{ kind: 'ok', value }` if preferred.

### Arrays and Tuples

Destack supports dynamic arrays, fixed-size arrays, and explicit tuple syntax:

```
int32[]                    // dynamic array
int32[N]                   // fixed-size array
readonly int32[]           // readonly array
(int32, boolean)           // tuple
(x: int32, y: int32)       // named tuple
readonly (int32, boolean)  // readonly tuple
```

The rules for arrays and tuples center around correctness and performance:
- Array literals are dense and do not permit holes.
- Index access `a[i]` returns the element type and is bounds checked.
- Bounds check failures follow the `boundsChecks` and `checkFailure` policies.
- `noUncheckedIndexedAccess` only affects index signatures, not arrays or tuples.
- Mutable arrays are assignable to readonly arrays.
- Readonly arrays are not assignable to mutable arrays.
- Readonly tuples are also not assignable to mutable tuples.
- Tuples are fixed-length value types and are assignable to arrays when their element types are compatible.

### References and Values

By default, a plain `T` follows the semantics of its type.
Structs and primitives are values.
Classes and structural object types are managed references.
Type aliases inherit the semantics of their underlying type.
Destack additionally supports explicit ownership control:

```
T            // type default (value or managed reference)
&T           // borrow (read only reference)
&mut T       // borrow (mutable reference)
^T           // owned reference (move-only)
^mut T       // owned reference (explicitly mutable)
```
Raw pointers are separate from ownership modifiers:
```
*T           // raw pointer (unsafe)
*mut T       // raw pointer (mutable, unsafe)
```

**Borrow semantics:**
- `&T` and `&mut T` are safe borrows verified by the borrow check pass.
- Borrows are created by `field.addr`, `element.addr`, and by calls that return borrowed references with lifetimes.
- `&expr` takes the address of an addressable place.
- When `expr` is not addressable, the compiler spills it to a temporary local and borrows that temporary.
- A borrow ends when the reference value is no longer live.
- Borrow checking uses liveness and alias analysis to detect conflicts and invalidations.
- Derived borrows carry provenance so dropping any origin invalidates the derived borrows.
- Dropping or freeing a value while it is borrowed is always an error.
- In strict mode, conflicting borrows and invalidating stores are errors.
- In lenient mode, the same situations produce warnings.

**Raw pointers:**
- `*T` and `*mut T` are unsafe pointers with no borrow tracking.
- Raw pointers may be null or dangling and allow pointer arithmetic.
- Converting between borrowed references and raw pointers is always explicit.
- Raw pointers do not imply ownership or drop behavior.
- Raw pointers only support equality and inequality comparisons.
- `&const T` and `*const T` are accepted but redundant and format as `&T` and `*T`.

| Modifier | Meaning | After `foo(x)` | Who cleans up? |
|----------|---------|----------------|----------------|
| `T` | Type default (value or managed reference) | `x` still valid | Type default |
| `&T` | Borrow (read) | `x` still valid | Original owner |
| `&mut T` | Borrow (mutate) | `x` still valid, maybe changed | Original owner |
| `^T` | Owned reference (move-only) | `x` **invalid** | New owner (raw allocation) |

Managed reference types are collected by the GC.
Value types only drop when owned or used with `using`.

**Implicit managed defaults:**
`noImplicitManaged` requires explicit ownership operators anywhere a type or value would otherwise use managed defaults.
Managed defaults include classes, interfaces, structural object types, arrays, functions, and string-like literals.
Explicit ownership operators (`^T`, `&T`, `*T`) satisfy the requirement for both type annotations and inferred values.
`noManaged` forbids GC-managed defaults and allocations.
Explicit ownership operators remain valid and use raw allocation.

Native and WASM outputs force strict defaults plus soundness defaults regardless of configuration.
The soundness defaults enforce:
- noAny.
- noImprecisePrimitives.
- noImplicitConversions and noUnsafeTypeAssertions.
- noImplicitManaged and noManaged.
- borrowMode = strict.
- noDynamicEvaluation, noDynamicImport, noProxy, noDynamicShapes, noExceptions, and noGlobalThis.

**Use after move:**

```
const node = AstNode { ... };
consume(^node);    // ownership transferred
print(node.value); // error: use after ownership transfer
```

`^T` is the owned reference type.
Passing a `^T` by value transfers ownership to the callee.
Use `^expr` to convert a value `T` into an owned reference `^T`.
If you already have `^T`, pass it directly instead of writing `^expr`.

Using a value after ownership transfer is an error (suppressible to warning).

**Drop as soon as possible:**

`^T` values are dropped at their last proven use (non lexical).
The compiler inserts a drop as soon as it can prove the value is no longer needed,
even if the lexical scope continues.

```
function process() {
    const data = ^LargeData { ... };  // we own this
    doWork(&data);                     // borrow it
    log("done");                       // data can be dropped before this line
}
```

**Drop behavior:**
- `Drop` is a marker interface that opts a type into last use cleanup.
- Types that implement `Drop` must also implement `Symbol.dispose`, which is invoked by the drop glue.
- `using` always calls `Symbol.dispose`, even without `Drop`.
- Owned values are dropped at their last proven use unless `using` is specified.
- `using` bindings drop at scope end and cannot be moved.
- `^T` controls ownership transfer and move semantics.
- `using` controls drop timing and does not imply ownership.
- Combine `using` with `^T` for deterministic cleanup of owned values.
- `using` can wrap managed values to enforce scope based cleanup when they implement `Symbol.dispose`.

**Allocation and drop:**

Ownership modifiers determine allocation strategy:

| Modifier | Allocation | Drop Instruction | Semantics |
|----------|------------|------------------|-----------|
| `T` | type default | none | value or managed reference |
| `^T` | owned storage | `raw.drop` | dispose + deallocate |
| `&T` | none | none | borrows existing memory |

Owned allocations use `raw.alloc` by default.
The `raw.drop` instruction performs **drop glue**:
1. Drop owned fields in reverse declaration order (LIFO)
2. Call `Symbol.dispose` if the type implements `Drop`
3. Deallocate the memory

The optimizer may promote `raw.alloc` to `stack.alloc` via escape analysis.
Stack allocated owned values use `stack.drop`, which runs the same drop glue but skips deallocation (the frame handles it).
For manual deallocation without dispose (FFI), use `raw.free` directly.

Optimization defaults to module scope for fast builds.
Target ltoMode selects the optimization scope.
Thin LTO runs at package scope and Full LTO runs at program scope.
Auto selects Thin LTO at O4 and disables LTO at lower levels.
Compilation unit refers to the selected optimization scope when LTO is enabled.

**Address spaces:**

References can target explicit address spaces for native and accelerator memory.
The default is `generic`, which maps to the target's normal memory.
Non generic address spaces are only valid for borrowed and raw references.
`constant` references are always immutable.
Address space changes are explicit and use the `addrspace.cast` intrinsic.
The VM may provide deterministic host side models for non generic address spaces, and otherwise rejects them with precise diagnostics.

**Nested ownership:**

Ownership is at the usage site, not the definition site.
Structs can contain `^T` fields regardless of how the struct itself is stored:

```
struct Container { data: ^Data; }

const value: Container = ...;          // value semantics container
const owned: ^Container = ...;         // owned container
```

When the container is `^Container`:
- Drop is deterministic
- Fields drop in reverse declaration order, then the container

When the container is boxed into managed storage:
- Drop is nondeterministic (GC finalizer)
- A warning is emitted in strict mode for `^T` fields in managed types

Strings follow the same ownership spectrum: `string` is GC managed by default,
`&string` is a borrowed view, and `^string` is explicitly owned.
Similarly, `Slice` is an explicit view type with a pointer and length.
Borrowing an array object does not imply a slice view.

Example (slice view):

```
function sum(values: Slice<int>): int {
    let total = 0;
    for (const value of values) total += value;
    total
}

const items = [1, 2, 3];
sum(items.as_slice());
```

### Ownership Conversions

Ownership conversions are explicit, except for borrows inserted at reference boundaries.
Implicit ownership conversions only create borrows and never transfer ownership.
Struct boxing at reference boundaries is a value to reference conversion, not an ownership conversion.

Implicit conversions:
- `T` → `&T` or `&mut T` when a reference is required and the value is addressable
- `^T` → `&T` to borrow from an owned value
- `&mut T` → `&T` to reborrow as shared

Explicit conversions:
- `T` ↔ `^T` require explicit ownership operators or helper calls (use `^expr` for `T` → `^T`)
- `&T` → `T` requires `Copy` or an explicit clone
- `&T` → `^T` requires an explicit clone and ownership transfer
- `*T` conversions require explicit unsafe operations

Example (implicit borrow):

```
function read(item: &Item): int { item.size() }

const value = Item { size: 10 };
read(value);
```

Example (explicit ownership conversion):

```
function consume(value: ^Item) { ... }

const value = Item { size: 10 };
consume(^value);
```

If you already have `^Item`, pass it directly without `^`.

Example (scope-end drop):

```
using file: ^File = File.open(path);
file.write("hello");
```

**Returning references:**

Functions can return `&T`.
Borrowed returns use lifetime inference and `@lifetime` annotations to track which inputs they borrow from.
In strict mode, returning a borrow that may outlive its origin is an error.
(In lenient mode, the same situation produces a warning).

```
function get(c: &Container): &Item { &c.item }  // ok

function bad(): &Point {
    const p = Point { x: 1, y: 2 }
    &p  // WARNING: returning reference to local
}
```

### Borrow Modes

By default, `&T` and `&mut T` are hints and violations produce warnings.
Strict mode enforces exclusive `&mut` borrows and no escape rules.
Strict mode enables stronger optimizations like `noalias` on `&mut`.
Enable strict mode with `borrowMode: "strict"` in `dsconfig.json`.
Borrow modes do not apply to raw pointers.

### Lifetime Annotations

When a function returns `&T` or a type containing borrowed references, the compiler tracks which input parameters the return value borrows from.
This is usually inferred automatically:

| Input Parameters | Inference |
|------------------|-----------|
| Single `&T` parameter | Return borrows from it |
| `&self` or `&this` receiver | Return borrows from receiver |
| Multiple `&T` parameters | Return borrows from all (conservative) |

When conservative inference is too restrictive, use `@lifetime` to specify exactly
which parameters the return borrows from:

```
@lifetime("param")          // borrows from single parameter
@lifetime("a", "b")           // may borrow from a or b
@lifetime("static")       // borrows from static/global data only (static is a reserved name anyway)
```

**Examples:**

```
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

```
function wrong(a: &string, b: &string): @lifetime("a") &string {
    return b;  // ERROR: return borrows from 'b', not 'a'
}
```

**Call site tracking:**

At call sites, the compiler uses lifetime information to check safety:

```
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

### Dynamic Parameterisation

Functions and methods work exactly like in JavaScript and TypeScript.
Function calls use positional arguments:

```
function myFunction(a: int, b: int) {
    ...
}

myFunction(2, 3);
```

Argument types are checked against the declared parameter types.
Parameters with defaults are optional at the call site.

### Static Parameterisation ("Generics")

Static parameters extend TypeScript-style generics with explicit comptime value parameters.
Type parameters accept type arguments and use TypeScript-style bounds.
Comptime value parameters accept static expressions and are constrained by value types.
Value parameters must be explicitly marked with `comptime` in the static parameter list; type annotations alone do not make a parameter a value parameter.

Static arguments are resolved during Analyze.
They must be static expressions and cannot require full comptime execution.
Static arguments may reference other static parameters.
Value parameter kinds are explicit and never inferred from usage.

Static value inference is limited and local:
- Static value arguments may be inferred from literal argument expressions in the current module when no explicit static argument is provided.
- Inference only uses fully static literal expressions: scalar literals, enum members, tuples, arrays, and objects.
- Non-static expressions, computed values, and cross-module inference do not participate.
- Literal array arguments may infer lengths when matching fixed-size array types like `T[N]`.
- Tuple literal arguments use Destack's tuple syntax `(a, b)` for inference.
- Enum members may be used only when the parameter type is that enum (or a union including it).
- Enum members do not implicitly coerce to their backing types.

For example, we can infer the size of an array from a literal argument:

```
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

```
struct Container<T: any> { // works like T extends any
    value: T
}

function identity<T>(x: T): T {
    x
}
```

Value parameters for compile-time constants:

```
function compute<comptime Flag: boolean>(data: uint8[]) {
    if Flag {
        ...
    }

    ...
}

compute<true>(); // pass the static argument positionally
```

Value parameters can drive type construction:

```
type Buffer<comptime N: number> = uint8[N];

declare let value: Buffer<4>;
value satisfies uint8[4];
```

### Where Clauses

Destack adds `where` clauses for type constraints beyond TypeScript's inline syntax.
Each clause is a type constraint of the form `Name: Type`:

```
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

## Comptime

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
Unlike Zig or Rust macros, however, Destack's comptime fills in well-defined **slots** rather than enabling fully arbitrary code generation.
Slots are typed because Analyze fixes all bindings and types before Execute.
In practice, this `comptime` behavior and specialisation together with decorators enable most macro-style use cases without the unpredictability and compiler complexity of a "full" macro system.

### Static Execution vs Comptime Execution

Destack uses two related but separate "evaluate during compile time" mechanisms:

- **Static execution** is evaluated during Analyze to produce `StaticExpression` values.
  This is a restricted subset of expressions that can be folded without executing user code.
  Static execution is required for static parameters, array sizes, and other type-driven
  constructs that must be known for analysis (i.e., type checking).
- **Comptime execution** evaluates `comptime` expressions and blocks during the Execute phase
  by running MIR in the VM interpreter. The result is substituted back into the program
  as a constant and any comptime-controlled branches are eliminated.

Static execution must not depend on full comptime execution.
This avoids dependency cycles between type resolution and code evaluation.

### Static If Decorators

The `@if(...)` decorator gates declarations and declaration members based on a static expression.
The condition must evaluate to a boolean using static execution.
The condition must not depend on static parameters.
Non-static conditions are a compile-time error.
When the condition is false, the annotated item is omitted from the symbol table for the active profile.
Multiple `@if` decorators are combined with logical AND.
The decorator is allowed on module declarations, class and struct members, interface members, and enum fields.

```
enum Os {
    @if(import.meta.platform == "windows")
    Windows,
    @if(import.meta.platform == "macos")
    Mac,
}
```

Use `if (comptime ...)` inside bodies for specialization that depends on static parameters.

### Import Meta

`import.meta` exposes per-profile metadata during static and comptime evaluation.
The values are fixed for the profile and are not runtime dependent.
`import.meta.output` is one of `js`, `ts`, `wasm`, or `native`.
`import.meta.runtime` is one of `browser`, `node`, `deno`, `bun`, `worker`, `wasm-js`, `wasm-wasi`, `native-hosted`, `native-freestanding`, or `native-embedded`.
`import.meta.platform` is one of `web`, `windows`, `macos`, `linux`, `ios`, `android`, `wasi`, `bare-metal`, or `universal`.
`import.meta.debug` is true in debug builds.
`import.meta.test` is true in test builds.
`import.meta.url` is the module URL and is always present.
`import.meta.path`, `file`, `filename`, `dir`, and `dirname` are file-system paths when available, otherwise `undefined`.
`import.meta.env` exposes profile-selected environment variables as a string map.

### Comptime Expressions

The `comptime` keyword wraps an expression or block, forcing compile-time evaluation:

```
// expression form
const VALUE: int = comptime 1 + 2 + 3;
const RESULT: int = comptime factorial(10);

// block form
const TABLE: uint8[] = comptime {
    let t = [];
    for (let i = 0; i < 256; i++) {
        t.push(computeCRC(i));
    }
    t
};
```
Comptime expressions infer their result type like any other expression.
Explicit annotations are recommended when the resulting type must be stable or obvious.
Comptime does not change the rules for static expressions, so static parameters still require static expressions.

The result is embedded as a constant in the compiled output.

### Comptime Blocks

Comptime blocks can appear as struct/class members or at module top-level.
They run during compilation rather than at runtime.

#### Member Comptime Blocks

Comptime blocks inside structs or classes run once per instantiation of the type.
They are useful for compile-time assertions on static parameters:

```
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

```
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

Static parameters support both type parameters and comptime value parameters.
Value parameters must be marked with `comptime` in the static parameter list.
The `comptime` modifier on parameters requires static evaluation during Analyze.
The `comptime` expression keyword evaluates later during Execute.

```
function repeat<comptime N: int>(value: string): string {
    let result = "";
    for (let i = 0; i < N; i++) { result += value; }
    result
}

const greeting = repeat<3>("hello ");  // N is comptime
```

Static parameters and `comptime` parameters serve similar purposes.
Use static parameters when the value affects the return type; use `comptime` parameters otherwise.

Static parameters require **static expressions**. They cannot depend on full comptime execution,
since static parameters are needed for instantiation and type resolution.
Defaults on static parameters apply when arguments are omitted.
Static arguments may reference other static parameters as long as the resulting expression remains static.

### Comptime Dynamic Parameters

Dynamic parameters can also be marked `comptime` to require compile-time-known arguments:

```
function createBuffer(comptime size: int): uint8[] {
    let buf = [];
    for (let i = 0; i < size; i++) { buf.push(0); }
    buf
}

createBuffer(1024);           // ok: literal is comptime-known
createBuffer(config.size);    // error: config.size not comptime-known

const N = comptime 1024;      // ok: redundant but valid
createBuffer(N);              // ok: N is comptime-known
```

Comptime parameters require **static expressions** as arguments. 
The call site may evaluate the function body via comptime execution, but the arguments themselves must be known during Analyze.

### Comptime Conditions

`if (comptime ...)` evaluates the condition as a static expression.
Type relations like `T extends U` are valid inside comptime conditions.
Both branches must type check even when the condition is statically known.

### Comptime Functions

Functions are not explicitly marked as "comptime" or not "comptime".
Any function can be called at compile time if its body is valid for comptime evaluation:

```
function factorial(n: int): int {
    if (n <= 1) { 1 } else { n * factorial(n - 1) }
}

// same function, different evaluation contexts
const COMPILE_TIME = comptime factorial(10);  // must be evaluated at compile time
const runtime = factorial(userInput);         // may be evaluated at runtime
```

The call site determines when the function runs, not the function definition.
A function that performs I/O cannot be called in a comptime context, but can still be called at runtime.

### Comptime Conditionals

When a condition is a comptime expression, the compiler can eliminate dead branches at compile time:

```
const DEBUG = comptime getEnvFlag("DEBUG");

function log(msg: string) {
    if (comptime DEBUG) {
        console.log(msg);
    }
}
```

When `DEBUG` is false, the entire `if` body is removed from the output.

### Comptime vs Static

TypeScript's `static` keyword and Destack's `comptime` keyword are orthogonal concepts:

| Keyword | Meaning | Example |
|---------|---------|---------|
| `static` (TS) | Belongs to class, not instance | `static count = 0` |
| `comptime` (DS) | Evaluated at compile time | `comptime factorial(10)` |

These compose naturally:

```
class Config {
    static DEFAULT = 256;                           // TS static, runtime initialization
    static LOOKUP = comptime generateLookupTable(); // TS static + comptime evaluation
}
```

A `static` block runs at class initialization time (runtime). 
A `comptime` expression runs during compilation (no runtime exists yet). 

### Comptime Type Conditions

When an `if (comptime ...)` condition is a type relation (`T extends U`):
1. The condition is evaluated at compile time
2. The type parameter is **narrowed** inside the true branch

```
function process<T, Context: CacheContext<T>>(ctx: Context, key: T) {
    if (comptime Context extends EvictableContext<T>) {
        // Context is narrowed to Context & EvictableContext<T>
        ctx.onEvict(key);  // valid: onEvict exists on EvictableContext
    }
}
```

The semantics of comptime type conditions matches TypeScript's conditional type semantics: `T extends U ? X : Y`.

| Condition | True Branch | False Branch |
|-----------|-------------|--------------|
| `comptime T extends U` | T narrowed to `T & U` | T unchanged |
| `comptime !(T extends U)` | T unchanged | T unchanged |
| `comptime T extends U \|\| ...` | T unchanged (complex) | T unchanged |
| `comptime T extends U && V extends W` | T → `T & U`, V → `V & W` | depends |

Narrowing only applies for simple `T extends U` conditions.
Complex boolean expressions do not narrow to avoid ambiguity.
Both branches of a comptime conditional must type check before comptime evaluation:

```
function example<T>(x: T) {
    if (comptime T extends Hashable) {
        x.hash();     // checked with T & Hashable
    } else {
        x.toString(); // checked with T
    }
}
```

#### Type<T> at Comptime

Reflection can be used at comptime, and thus `Type` can also be used for comptime conditionals:

```
function serialize<T>(value: T): string {
    if (comptime Type<T>.kind == "struct") {
        // T is known to be a struct
        return comptime generateStructSerializer<T>();
    } else if (comptime Type<T>.kind == "array") {
        return comptime generateArraySerializer<T>();
    } else {
        return JSON.stringify(value);
    }
}
```

### Comptime "Slots"

Destack's comptime is designed around the concept of **slots**:

1. **Types are fixed during Analyze**: all bindings, symbols, and types are determined.
2. **Comptime expressions are slots**: positions where a value will be computed.
3. **Execute phase fills the slots**: computes values, eliminates dead branches.
4. **No binding changes**: the symbol table never changes after Analyze.

#### Code Specialization

Comptime cannot generate arbitrary code or perform dynamic evaluation.
Dynamic evaluation is a runtime feature and is not available during comptime.
But, we can specialize code statically to cover essentially all relevant use cases via comptime specialization, which is more maintainable and understandable anyway.
In practice, this isn't its own "feature", but just a nice consequence of other orthogonal features:

1. **Monomorphization**: generic functions become specialized per type argument.
2. **Loop unrolling**: iteration over comptime known collections (like `Type<T>.fields`) is unrolled.
3. **Branch elimination**: comptime conditionals select which code survives.
4. **Inlining**: comptime expressions become constants.

#### Comptime Example: Cache Table

This example demonstrates comptime type conditions for conditional behavior.
See [Ghostty's cache_table.zig](https://github.com/ghostty-org/ghostty/blob/main/src/datastruct/cache_table.zig) for the original Zig implementation.

```ds
/// Cache context interface
interface CacheContext<K> {
    /// Hash the key to a uint64.
    hash(key: K): uint64;
    /// Check if two keys are equal.
    equal(a: K, b: K): boolean;
}

/// Cache context interface that supports eviction.
interface EvictableContext<K, V> extends CacheContext<K> {
    /// On eviction of a key and value.
    evicted(key: K, value: V): void;
}

/// Fixed size arrays from static parameters.
export struct CacheTableKV<K, V> { 
    key: K, 
    value: V 
}

/// Cache table with static (comptime) parameters.
export struct CacheTable<
    K,
    V,
    Context: CacheContext<K>,
    comptime bucketCount: uint16,
    comptime bucketSize: uint16,
> {
    /// Comptime block for compile-time assertions.
    /// Runs once per instantiation of this generic struct.
    comptime {
        assert(
            (bucketCount & (bucketCount - 1)) == 0,
            `bucketCount must be power of 2, got ${bucketCount}`
        );
    }

    /// Associated type alias for the key-value pair type.
    type KV = CacheTableKV<K, V>;

    /// KV pairs for the buckets.
    buckets: KV[bucketSize][bucketCount],
    /// Lengths of the buckets.
    lengths: uint8[bucketCount] = comptime [0] * bucketCount,
    /// Context for the cache.
    context: Context,

    /// Put a key and value into the cache.
    put(key: K, value: V): KV | null {
        const kv = KV { key, value };
        const idx: uint = this.context.hash(key) % bucketCount;

        if (this.lengths[idx] < bucketSize) {
            this.buckets[idx][this.lengths[idx]] = kv;
            this.lengths[idx] += 1;
            return null;
        }

        const evicted = rotateIn(&this.buckets[idx], kv);

        // comptime type guard (equivalent to Zig's `@hasDecl`)
        if (comptime Context extends EvictableContext<K, V>) {
            // context is narrowed: this.context.evicted() is valid
            this.context.evicted(evicted.key, evicted.value);
        }

        evicted
    }

    /// Get a value from the cache.
    get(key: K): V | null {
        const idx: uint = this.context.hash(key) % bucketCount;
        const len = this.lengths[idx];

        for (let i = len; i > 0; i--) {
            if (this.context.equal(key, this.buckets[idx][i - 1].key)) {
                const value = this.buckets[idx][i - 1].value;
                rotateOnce(this.buckets[idx].slice(i - 1, len - 1));
                return value;
            }
        }

        null
    }

    /// Clear the cache.
    clear(): void {
        // report eviction of all values if the context is evictable
        if (comptime Context extends EvictableContext<K, V>) {
            for (const [bucket, length] of zip(this.buckets, this.lengths)) {
                for (const kv of bucket.slice(0, length)) {
                    this.context.evicted(kv.key, kv.value);
                }
            }
        }

        this.lengths.fill(0);
    }
}
```

## Interactive Execution

Destack defines an interactive execution profile for REPLs and notebooks.

### REPL Profile

A REPL session evaluates each cell as a synthetic module.
Each cell module is compiled through the full pipeline and executed in a persistent VM isolate.
The REPL profile implicitly imports a session prelude that reexports earlier cell exports.
Top-level bindings are implicitly exported unless configured otherwise.
The last expression in a cell is captured as the cell result value.
Top-level await is permitted in the REPL profile.

### Dynamic Evaluation

Dynamic evaluation is a runtime feature and is never available at comptime.
When allowed by configuration, `eval` and `Function` are available with JS compatible semantics on JS targets.
On native and VM targets, dynamic evaluation compiles the input into a synthetic module under the dynamic execution policy.
Dynamic evaluation returns `any` by default and should be narrowed explicitly.
Dynamic evaluation is gated by a policy in the project configuration and may be disabled entirely in production.

## Reflection

Destack makes types first-class runtime values, enabling reflection without separate metadata systems or configuration.
This section specifies the built-in reflection capabilities; the standard library `@destack-sh/schema` extends these with validation utilities.

### Type Descriptors

Every nominal type `T` has a corresponding runtime descriptor value of type `Type<T>`.
For classes and structs, the constructor value doubles as the descriptor, so it is both constructable and reflective.
Using a type name in value position evaluates to that descriptor value:

```
struct Point { x: float32, y: float32 }

const t = Point;              // t: Type<Point>
const t: Type<Point> = Point; // explicit annotation
```

The `typeOf` function returns a descriptor for a value's type (unlike the runtime `typeof`, which returns a coarse-grained string like `"object"`).
The type-level `typeof` operator returns the value type of an expression, including constructor signatures and static members for classes and structs:
The `typeof` operator is type-only, and type aliases are not values in expression position.

```
const p = Point { x: 1, y: 2 };
const t = typeOf(p);          // Type<Point>
```

### Runtime Type Identity

Runtime type identity (RTTI) is demand-driven.
The compiler only emits RTTI for types that are used at runtime.
Examples include `typeOf`, `T.is`, `x is T`, `instanceof` for classes, `any`/`unknown`, and runtime reflection.
Polymorphic classes carry a vtable pointer for dynamic dispatch and RTTI.
Non-polymorphic classes may omit the vtable pointer and rely on metadata or fat pointers when RTTI is required.
Structs are pure data unless RTTI is required by usage.
On JS targets, RTTI-enabled values use a hidden symbol property rather than a global WeakMap, preserving "plain object" semantics.
Native type tags are `TypeTag` handles that point to `TypeDescriptor` values.
Classes reach RTTI via vtable slot 0 when present, while thin pointers without tags recover RTTI via GC metadata.

### Runtime Type Guards

Type descriptors expose `T.is(value)` for runtime type checks.
The `x is T` operator is syntactic sugar for `T.is(x)` when RTTI is required.
The `instanceof` operator checks class identity and is only defined for class types.
Constructable signatures do not make a non-class type a valid `instanceof` target.
For structural or non-class types, use `x is T` instead.

### Decorator Metadata

Decorator information is accessible at runtime on-demand:

```
@deprecated("use newAPI")
function myOldMethod() { }

myOldMethod.decorators  // [{ name: "deprecated", args: ["use newAPI"] }]
```

### Standard Library

The built-in `Type<T>` interface provides basic reflection.
The standard library `@destack-sh/schema` extends it for general schema use:

```
// Built-in (always available with Reflection feature)
Point.name        // "Point"
Point.fields      // [{ name: "x", ... }, { name: "y", ... }]
Point.is(value)   // type guard

// Standard library (requires import)
import { parse } from "@destack-sh/schema";
parse(Point, data)      // runtime validation
Point.parse(data)       // shorthand via extension
```

## Declarations

Declaration forms in Destack match TypeScript, with the addition of richer static parameterisation and our concept of nominal typing (like with `newtype` behavior for `enum`s).

### Interface

Interfaces work like TypeScript, with optional default functions and properties:

```
interface Drawable {
    draw(): void

    isVisible(): boolean {
        true  // default implementation
    }
}

interface Container<T> extends Iterable<T> {
    static Empty: this
    size(): uint64
    get(index: uint64): T | undefined
}
```

#### Nominal Interfaces

Destack adds **nominal interfaces** using the `newtype interface` syntax.
Nominal interfaces require explicit ("nominal") `implements` declarations.

```
// Structural interface (standard TypeScript behavior)
interface Drawable {
    draw(): void
}
const x: Drawable = { 
    draw() { } 
};  // OK: structural match

// Nominal interface (requires explicit "nominal" `implements`)
newtype interface Add<T, R = this> {
    add(other: T): R
}

struct Vec2 { x: float, y: float }
const v: Add<Vec2> = Vec2 { x: 1, y: 2 }  // ERROR: Vec2 doesn't implement Add

// Explicit opt-in required
extension for Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 { Vec2 { x: this.x + other.x, y: this.y + other.y } }
}
const v: Add<Vec2> = Vec2 { x: 1, y: 2 }  // OK: Vec2 implements Add
```

Nominal interfaces are used for:

- **Operator interfaces** (`Add`, `Compare`, `Equal`, etc.) to prevent accidental operator overloading
- **Marker traits** (`Send`, `Sync`, `Copy`) for compile-time capabilities

The `newtype` modifier follows the same pattern as `newtype` on type aliases—it makes the interface nominal.
Extending a nominal interface produces a nominal interface (nominality is inherited).

### Class

Classes work like TypeScript.
Classes are reference types with identity and prototype-based inheritance:

```
class MyClass {
    field: int32;

    constructor(value: int32) {
        this.field = value;
    }
}
```

#### Constructors

Classes and structs can declare a `constructor` method.
If no constructor is declared, a default constructor exists.
The default constructor takes positional arguments in field declaration order and assigns them to fields.
Constructors cannot return a value.
A bare `return` is allowed for early exit.
Constructors must initialize all instance fields before returning.
The `new` expression always invokes the constructor (explicit or default).
For class types, `new` allocates a managed instance and runs the constructor.
For struct types, `new` constructs a value and does not imply managed allocation.
In struct constructors, `this` is the value under construction, not a managed reference.
Assignments to `this` fields initialize the value payload directly.
Struct literals directly initialize fields without running constructors.

Classes have **identity**: two instances are only `===` if they're the same object:

```
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

```
struct Point {
    x: float32;
    y: float32;
}
```

#### Struct vs Class

| | struct | class |
|---|---|---|
| Reference identity | ❌ No (`===` is error) | ✅ Yes (`===` compares pointers) |
| Inheritance | ❌ No (use embedding) | ✅ Yes (`extends`) |
| Default passing | Value | Reference |
| Default storage | Inline | Managed reference |
| JS output | Plain object | ES6 class |

Structural object types (`type X = { ... }` and inline `{ ... }`) are reference types with identity.
Type aliases inherit the semantics of the underlying type.
Struct values can be boxed when a reference type is required.

#### Equality and Identity

Structs have no reference identity, so two structs with the same properties are equal by value.
Structs auto-derive `Equal` (field-by-field comparison) by default:

```
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
p1 == p2;   // true: same fields = equal (auto-derived Equal)
p1 === p2;  // error: === requires reference identity, structs have none
```

Since structs have no reference identity, `===` and `!==` are compile errors on struct types.
Use `==` for value comparison.

#### Construction

Structs are nominal, so they must be explicitly constructed:
Tooling may lint `new` on structs in favor of `Point { ... }`.

```
let p: Point = Point { x: 1, y: 2 };  // ok: explicit construction
let p: Point = new Point(1, 2);       // ok: constructor syntax
let p: Point = { x: 1, y: 2 };        // error: object literal is not Point
```

#### Pattern Matching

Struct patterns require the type name (unlike newtypes which auto-unwrap):

```
match (point) {
    Point { x: 0, y: 0 } => "origin";
    Point { x, y } => `at ${x}, ${y}`;
    { x, y } => ...;  // error: structural pattern on nominal type
}
```

#### Interfaces and Composition

Structs can `implements` interfaces but cannot `extends` (use embedding instead):

```
struct Point implements Drawable {
    x: float32;
    y: float32;

    draw(): void { ... }
}

struct Transform { position: Vec3; rotation: Quat; }
struct Player {
    ...Transform;    // embeds Transform's fields (composition)
    health: int;
}
```

At runtime in JS, a struct is a plain object.
Its nominal type is erased.
Ownership (`&T`, `^T`) is orthogonal to identity semantics.
You can explicitly reference or copy either structs or classes.

#### Associated Types

Class-shaped types like structs and interfaces can declare associated type aliases ("static type members") using the `type` keyword:

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
Associated types are resolved at compile time based on the type's static parameters.

Associated types can be accessed via the containing type:

```
const item: Container<int>.Item = 42;  // Item resolves to int
```

In interfaces, associated types can be declared without a default (abstract) or with a default:

```ds
interface Iterable<T> {
    type Item;                    // abstract: implementors must provide
    type Iter: Iterator<Item>;    // abstract with constraint

    iter(): Iter;
}

extension<T> for Container<T> implements Iterable<T> {
    type Item = T;
    type Iter = ContainerIterator<T>;

    iter(): Iter { ... }
}
```

### Enum

Enums conceptually follow TypeScript, but remain strictly nominal types to avoid accidental implicit coercions.
Enum values do _not_ implicitly coerce to their backing type.
Explicit casts are required to convert between enums and their backing types.
The backing type is inferred from member values and is either an integer or string type.
When member values are omitted, the backing type defaults to the configured integer width.

Enum member values are constant expressions.
Integer-backed enums allow constant integer expressions using literals, unary +/-, binary arithmetic or bitwise operators, casts, parentheses, and references to earlier enum members.
Integer-backed enums assign implicit values starting at zero, and explicit values advance the next implicit value by one.
String-backed enums require explicit string values for every member.

```
enum Status {
    Active
    Inactive
    Pending
}
```

Because enums are effectively newtypes, they can also carry methods and constants:
```
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

```
extension for Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }

    normalized(): Vector2 {
        const m = this.magnitude();
        Vector2 { x: this.x / m, y: this.y / m }
    }
}
```

#### Extensible Types

Extensions require **nominal types**—types with declaration identity:

| Type | Nominal? | Extensible? |
|------|----------|-------------|
| `struct S { }` | ✅ | ✅ |
| `class C { }` | ✅ | ✅ |
| `enum E { }` | ✅ | ✅ |
| `newtype N = T` | ✅ | ✅ |
| `int32`, `string`, ... | ✅ (prelude) | ✅ |
| `type X = T` | ❌ (alias) | ❌ |
| `{ x: T }` inline | ❌ (structural) | ❌ |
| `T | U` union | ❌ (structural) | ❌ |

To extend a structural shape, wrap it in a nominal type:

```
// can NOT extend type aliases or inline types (structural)
type Point = { x: number, y: number };
extension for Point { ... }  // error: Point is a type alias

// can extend newtype or struct (nominal)
newtype Point = { x: number, y: number };
extension for Point { ... }  // ok: Point is nominal
```

Unlike Rust's blanket `impl`s, Destack does not support generic extensions like `extension<T> T where T: Constraint`.
Extensions target concrete nominal types only to keeps extension resolution predictable in TypeScript's expressive type system.

Extension static parameters bind positionally to the target type's static parameters.
The target type expression can reorder those parameters (e.g., `extension<Left, Right> for Pair<Right, Left>`), and that order defines how receiver static arguments map to extension parameters.
Defaults on the target type apply when static arguments are omitted at the use site.

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

```
newtype UserId = int;
extension for UserId {
    isValid(): boolean { this > 0; }
}

// anonymous extension on builtin type: only visible in this file
extension for int32 {
    abs(): int32 { if (this < 0) { -this } else { this }; }
}
```

Since `UserId` is defined in the same file, its extension is visible wherever `UserId` is used.
Since `int32` is a builtin (foreign) type, the anonymous extension is only visible in this file.
(Destack includes a prelude for builtin types that is automatically imported.)

#### Implementing Interfaces

Extensions can implement interfaces, enabling operator overloading:

```
interface Add<T, U = T> {
    add(other: T): U;
}
```

```
extension for Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}

// now you can use: v1 + v2
```

Multiple overloads for the same operator are supported via multiple interface implementations:

```
extension for Vector2 implements Add<Vector2>, Add<float> {
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

```
import { Vector2 } from "somewhere";

// anonymous: only visible in this file (foreign type)
extension for Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

This anonymous extension on `Vector2` is only usable in the file where it's defined.
To share extensions on foreign types, use named extensions and import them.

#### Named Extensions

Named extensions can be exported and must be imported where used:

```
// in date-utils.ds
import { Date } from "builtin";

export extension DateUtils for Date implements Add<Date> {
    addDays(days: int): Date { ... }

    add(other: Date): Date { ... }
}
```

```
// in app.ds
import { DateUtils } from "./date-utils.ds";

const tomorrow = today.addDays(1);  // works: DateUtils is imported
```

Without importing `DateUtils`, the `addDays` method is not available even if `Date` is in scope.

### Function

Functions work like TypeScript, with the addition that the last expression is implicitly returned.

#### Basic Functions

Functions are dynamically and statically parameterised pieces of reusable logic, just like in JavaScript (and TypeScript).

```
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

```
async function fetchData(url: string): Promise<Response> {
    const response = await fetch(url);
    response
}
```

#### Generator Functions

Generator functions use `function*` and `yield`:

```
function* range(start: int32, end: int32): Generator<int32> {
    for i in start..end {
        yield i
    }
}

function* fibonacci(): Generator<int32> {
    let (a, b) = (0, 1);
    loop {
        yield a;
        (a, b) = (b, a + b);
    }
}
```

#### Lambda Expressions

Arrow functions work like TypeScript:

```
(x) => x * 2
(a: int32, b: int32): int32 => a + b
(items) => {
    for item in items {
        process(item)
    }
}
```

#### Function Overloading

Destack supports function overloading without the clumsy type-only declarations:

```
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

##### Overload Resolution

Overloads are resolved using **declaration order**: the first matching overload wins.
This matches TypeScript's overload resolution semantics.

```
// Good: specific overloads before general ones
function format(x: "json"): JsonFormatter;
function format(x: "xml"): XmlFormatter;
function format(x: string): Formatter;

format("json")    // calls first overload
format("xml")     // calls second overload
format("csv")     // calls third overload
```

```
// Bad: general overload shadows specific ones
function format(x: string): Formatter;
function format(x: "json"): JsonFormatter;  // warning: shadowed by first overload

format("json")    // calls first overload (not second!)
```

The compiler warns when an overload is shadowed by an "earlier" declaration that always matches first.
For union argument types, the argument must be assignable to a single overload:

```
function handle(x: string): string;
function handle(x: number): number;

const y: string | number = getValue();
handle(y);  // error: union argument not assignable to any overload
```

```
function handle(x: string | number): string | number;

const y: string | number = getValue();
handle(y);  // ok
```

##### Dynamic Resolution on Unions

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

##### Dispatch Tables

Classes use vtables for virtual method dispatch.
VTables are emitted only when virtual dispatch remains after devirtualization.
VTable slot 0 stores the type tag when RTTI is enabled.
VTable slot 1 stores the drop glue function.
Virtual methods follow in declaration order, and overrides reuse the same slot.
Interface dispatch is separate and uses itabs instead of class vtables.
Itabs are generated for both struct and class implementations.
Each itab is specific to a (Type, Interface) pair.
Interface references are fat pointers carrying an object pointer and an itab pointer.
Itab slot 0 stores the type tag when RTTI is enabled.
Itab slots follow interface member declaration order, including fields and methods.
Interface inheritance flattens base interfaces in extends list order before local members.
Members inherited with the same name and signature reuse the first slot.
Fields reuse slots only when their declared types match.
Conflicting member signatures are errors during analysis.
Each interface field contributes a field offset slot.
Each interface method contributes a method target slot.
Static members are not part of vtables or itabs.
Static methods and properties lower to direct symbols.

#### Methods

Methods are functions declared inside types (structs, classes, enums, interfaces, extensions).
They receive `this` as an implicit first parameter:

```
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

Methods can declare an explicit `this` parameter to constrain the receiver type.
The explicit `this` parameter must be first and does not count toward call arity.
Explicit `this` parameters can use reference types (like `&mut`) to require mutable receivers.

Methods can also be added to any type via extensions, including primitives and foreign types.

#### Getters and Setters

Getters and setters work like TypeScript:

```
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

```
class Person {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}
```

#### Rest Parameters and Spread

Rest parameters and spread operators work like TypeScript:

```
function sum(...numbers: int32[]): int32 {
    numbers.reduce((a, b) => a + b, 0)
}

sum(1, 2, 3, 4, 5);

const args = [1, 2, 3];
sum(...args);
```

### Namespace

Namespaces work like in TypeScript:

```
namespace math {
    export const PI = 3.14159
    export function sin(x: float64): float64 { /* ... */ }
}
```

### Visibility

Visibility modifiers work like TypeScript:

```
public field: int32
private field: int32
protected field: int32
#field: int32        // shorthand for private
```

## Expressions

**Everything is an expression** in Destack.
Blocks, `if`, `match` all return values:

```
const result = if x > 0 { "positive" } else { "negative" };
const label = match (state) {
    Ready => "go"
    Loading => "wait"
};
```

### Bindings

Variable bindings work like TypeScript, with Destack adding tuple destructuring syntax:

```
const x = 1;              // immutable
const x: int32 = 1;       // with type
let y = 2;                // mutable

const [a, b] = getTuple();
const { x, y } = getPoint();
const (a, _) = getTuple();  // Destack tuple syntax
```

Unlike JavaScript/TypeScript, bindings can be re-declared in the same scope with a different type (like Rust):

```
const x = "hello";        // x is string
const x = x.length;       // x is now int (shadowing)
```

### Blocks

Block expressions group multiple statements and return the last expression's value.
Blocks are enclosed in `{ }` and can optionally have labels:

```
const result = {
    const x = compute();
    const y = transform(x);
    x + y                    // last expression is the block's value
};
```

Labeled blocks allow breaking with values:

```
const value = outer: {
    for i in 0..100 {
        if condition(i) {
            break outer i;    // return i from the block
        }
    }
    -1                       // default if no break
};
```

For disambiguation (e.g., after `if` or `match`), use `do`:

```
const x = if flag { do { compute() } } else { 0 };
```

### Conditionals

Control flow with `if` works like TypeScript.
Unlike TypeScript, `if` is an expression that returns a value.

```
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

```
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

Match must be exhaustive—all possible values must be handled, or use `_` as a catch-all.
The match expression type is the union of its case body types.
`break` is not allowed inside `match` arms.

TypeScript's `switch` also works unchanged in syntax, but it is a statement like expression that yields `void` in Destack.
Switch cases fall through by default and require `break` to stop.
`break` in a `switch` cannot carry a value.

```
switch (value) {
    case 0:
        return "zero"
    case 1:
        return "one"
    default:
        return "other"
}
```

### Loops

Destack supports all TypeScript loop forms plus `loop`.

#### For-Of and For-In

Iterate over iterables with `for...of` (values) or `for...in` (keys), matching TypeScript exactly:

```
for (const item of items) {
    process(item)
}

for (const key in object) {
    print(key)
}

// with range literals
for (const i of 0..10) {
    print(i)          // 0, 1, 2, ..., 9
}

for (const i of 0..=10) {
    print(i)          // 0, 1, 2, ..., 10 (inclusive)
}

// with destructuring
for (const [key, value] of map) {
    print(`${key}: ${value}`)
}
```

#### While

Standard while loops work like TypeScript:

```
while (condition) {
    process()
}

// do-while
do {
    process()
} while (condition)
```

#### Loop

Infinite loop that can be exited only with `break`:

```
loop {
    const input = readInput();
    if (input == "quit") {
        break;
    }
    process(input);
}
```

#### Traditional For

TypeScript's traditional for loop also works:

```
for (let i = 0; i < 10; i++) {
    print(i)
}
```

#### Labeled Breaks

All loop forms support labeled breaks and continues:

```
outer: for (const i of 0..10) {
    for (const j of 0..10) {
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

```
_                    // ignore this value
```

#### Binding

Captures a value into a variable:

```
x                    // bind to x
^mut x                // bind to mutable x
```

#### Must

Matches any non-nullish, Try-unwrapped value and binds it:

```
x!                   // bind only if value is not null or undefined
```

#### Literal

Matches exact values:

```
42                   // match integer
"hello"              // match string
true                 // match boolean
```

#### Tuple

Matches tuple structure:

```
(a, b)               // two-element tuple
(x, _, z)            // ignore middle element
(first, ...rest)     // rest pattern
```

#### Object

Matches object/struct properties:

```
{ x, y }             // shorthand
{ x: a, y: b }       // rename bindings
{ x, ...rest }       // rest pattern
Point { x: 0, y }    // tagged with literal field
```

Tagged object patterns accept any object-like type expression, including type aliases and interfaces.
The tag must resolve to an object type, or the pattern is a type error.

#### Array/Slice

Matches array elements:

```
[a, b, c]            // exact three elements
[first, ...rest]     // first + rest
[first, ..., last]   // first and last
[]                   // empty array
```

#### Variant

Matches enum/union variants:

```
Some(x)              // unwrap Some
Ok(value)            // unwrap Ok
Result.Err(e)        // qualified path
```

#### Range

Matches numeric ranges:

```
1..10                // exclusive range
1..=10               // inclusive range
```

#### Union

Matches any of several patterns:

```
1 | 2 | 3            // match 1, 2, or 3
"a" | "b"            // match either string
```

#### Guards

Add conditions to patterns (only in `match`):

```
n if n > 0           // positive numbers only
x if x.isValid()     // with method call
```

### Errors and Exceptions

Destack uses **Result-first error handling**: recoverable errors use `Result<T, E>`, while `throw` is reserved for unrecoverable panics.
The builtin `Error` interface is the conventional error shape, but any type can be used as `E`.
For compatibility with existing JS/TS, we do support `throw` in JS targets.

#### Try protocol

The `Try<T, E>` interface defines the protocol for `?` and `??`.
`Try` is nominal, while its branch shape is structural.
`Try.branch()` must return a `TryBranch<T, E>` compatible shape:

```
type TryBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };
```

Structural compatibility refers to structural object types; nominal types like `struct` and `newtype` do not implicitly satisfy `TryBranch`.
Use object shapes or type aliases for branch values when implementing `Try`.

Implementations must provide `Try.fromError(error: E): this` for uncaught early returns.
`Try.fromError` is not required when a `?` is inside a `try` with `catch`.
`Try.fromError` is not required for `??` because it does not return early.

#### Try and the ? Operator

The `?` operator propagates `Try` errors to the caller:

```
function readConfig(path: string): Result<Config, Error> {
    const text = readFile(path)?    // returns early if Err
    const json = parseJson(text)?   // returns early if Err
    Result.ok(Config.from(json))
}
```

When `?` is applied to a `Try<T, E>`:
- If the branch is ok, extracts and returns the success value.
- If the branch is err, returns early with `Try.fromError(error)` from the enclosing function.
The receiver must be a non-nullish `Try` type.
Unions containing non-`Try` or nullish members are not valid operands for `?`.
The success type is returned as-is without stripping nullish values.
The `?` operator unwraps at most one `Try` layer.
For unions of `Try` types, the success type and error type are both unioned.

The enclosing function must return a compatible `Try` type.
The error type `E` is unconstrained, but `Error` is the conventional shape.
Non-`Error` error types should emit a lint, not a hard error.

#### Try and the ?? Operator

The `??` operator extracts the success value or uses a default:

```
const config = loadConfig() ?? defaultConfig;
const port = parsePort(input) ?? 8080;
```

When `??` is applied to a `Try<T, E>`:
- If the branch is ok, extracts and returns the success value.
- If the branch is err, returns the right-hand default value.

When `??` is applied to `T | null | undefined`:
- If nullish, returns the right-hand default value.
- Otherwise, returns the left-hand value.

Evaluation order is fixed and does not depend on union ordering:
1. Evaluate the left-hand side
2. If the value is nullish, return the right-hand side
3. Else if the value implements `Try`, branch and return the success value or the right-hand side on error
4. If the resulting value is nullish, return the right-hand side
5. Otherwise, return the value as-is

`??` performs at most one `Try` unwrap.
When the left-hand side is a union of `Try` and non-`Try` values, the result unions the unwrapped success types, non-`Try` members, and the fallback.
Nullish values are removed from both the union and the `Try` success type before the result is formed.

```
const value: Result<User, IOError> | null = loadUser();
const user = value ?? defaultUser;  // default on null or Err
```

#### try/catch with Result and exceptions

The `try`/`catch` syntax handles exceptions and explicit `Try` propagation:

```
try {
    const config = readConfig("config.json")?;
    process(config);
} catch (e: IOError) {
    log("Failed:", e)
} finally {
    cleanup()
}
```

The example uses `Result`, but any type implementing `Try` behaves the same.
`try` does not implicitly unwrap `Result` values.
Use `?` or `??` inside the block to propagate `Try` errors into the catch.
When a `?` is inside a `try` with a catch, `Try.fromError` is not required.
Exceptions still propagate into the catch on JS targets, or are rejected by `no_exceptions` on native.
A try expression must include a catch or finally block.

```
try {
    riskyOperationA()?;
} catch match e {
    NumericError(x) => Error(@format("bad number: {x}"))
    FormatError => Error(@format("bad format {e}"))
    _ => Error(@format("unknown error: {e}"))
}
```

#### Panic

`throw` is intended for **unrecoverable errors**: assertion failures, invariant violations, bugs.
Panics indicate programmer error, not conditions the caller should handle.

```
function unwrap<T>(r: Result<T, Error>): T {
    match (r) {
        Ok { value } => value
        Err { error } => throw error   // panic: caller made a mistake
    }
}
```

**Native targets:** `throw` aborts the process immediately. No stack unwinding, no catching.
**JS targets:** `throw` behaves as normal JavaScript throw for compatibility.

### Using
`using` declares a resource that will be disposed when the current **lexical scope** exits.
Destack mirrors JS/TS semantics and syntax, following the TC39 Explicit Resource Management proposal.

```
using file = openFile(path);
await using conn = openConnection();
```

#### Declarations
`using` and `await using` appear anywhere a lexical declaration is allowed.
They always have an initializer and are immutable like `const`.
`await using` is only allowed where `await` is legal.
`export using` is valid and behaves like `export const`.

```
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

```
using first = getFirst();
using second = getSecond();
```

#### Type requirements
`using` requires `Disposable | null | undefined`.
`await using` requires `AsyncDisposable | Disposable | null | undefined`.
Types implementing `Drop` satisfy `Disposable` implicitly.
On JS targets, `Drop` lowers to a `[Symbol.dispose]()` wrapper.

```
struct File implements Drop {
    drop(): void { close(this) }
}

using file: ^File = File.open(path);
```

#### Loops
In `for`/`for..of`/`for..in` initializers, a `using` resource is per-iteration and disposed at the end of each iteration.
The disposal runs on `continue` and `break`, just like normal scope exit.

```
for (using line of readLines(path)) {
    process(line);
}
```

#### Modules
Top-level `using` in modules disposes when module evaluation completes.
This includes completion after top-level `await`.

```
using log = openLog();
await run();
```

#### Drop and ownership
In `.ds` files, `^T` values that implement `Drop` are disposed at their last proven use.
`using` pins disposal to scope exit even if the value would otherwise drop earlier.
`using` bindings are not movable, so `^x` from a `using` binding is an error.
Use `^T` without `using` when you want eager drop.

```
using buffer: ^Buffer = allocate();
use(&buffer);
```

## Operators

Destack operators match TypeScript with additional precision for integer arithmetic.
Many operators can be overloaded via extensions implementing the corresponding interface.
Operators with an interface in the table below desugar to method calls on the left operand (receiver-based dispatch).

The receiver type determines which implementation "family" to look at, and the right operand type selects the specific overload within that family.
For example, `Vector2` can implement both `Add<Vector2>` and `Add<float>` for vector addition and scalar addition respectively.

### Explicit Operator Overloading

Operator interfaces are declared as **nominal interfaces** using `newtype interface`:

```
newtype interface Add<T, R = this> {
    add(other: T): R
}
```

Because they are nominal, operators only overload when a type explicitly declares `implements` for the operator interface.
(This prevents accidental conformance from types that happen to have a structurally-compatible method.)

```
// Foo has an `add` method but doesn't implement Add<T>
struct Foo {
    add(other: Foo): Foo { ... }
}

const a = Foo { };
const b = Foo { };
a + b;        // error: Foo does not implement Add
a.add(b);     // ok: direct method call works

// Foo explicitly implements Add<T>
extension for Foo implements Add<Foo> {
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
Explicit per check settings override the preset.
The safety preset does not modify `checkFailure`.

```
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
| `as` | Type cast | — |
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
The `extends` and `implements` operators return boolean literals when assignability is decidable and `boolean` otherwise.

Conditional types allow `infer` bindings inside the `extends` pattern.
The inferred bindings are scoped to the conditional type and available in the true branch.

Repeated `infer` bindings union candidates in covariant positions and intersect candidates in contravariant positions (such as function parameters).

Conditional types distribute over unions only when the left side is a naked type parameter.
Distributive conditionals treat `never` as an empty union and evaluate to `never`.

When the checked type is `any`, the result is the union of the true and false branches.
Wrapping the type parameter (for example, in a tuple) disables distributive behavior.

When `never` is matched against an `infer` pattern, inference yields `never` for structural and template literal patterns.

### Mapped Types

Mapped types construct new object types by iterating over keys.
They follow TypeScript semantics and are primarily used by utility types like `Partial` and `Readonly`.

```
type Flags<T> = { [K in keyof T]: boolean };
type Optional<T> = { [K in keyof T]?: T[K] };
type Required<T> = { [K in keyof T]-?: T[K] };
type Frozen<T> = { readonly [K in keyof T]: T[K] };
type Mutable<T> = { -readonly [K in keyof T]: T[K] };
```

Key remapping is supported with `as`:

```
type Renamed<T> = { [K in keyof T as "value"]: T[K] };
```

### Utility Types

Destack provides the standard TypeScript utility types via the builtin libs.
They are defined using mapped and conditional types and follow TypeScript semantics:

- `Partial<T>`, `Required<T>`, `Readonly<T>`
- `Pick<T, K>`, `Omit<T, K>`
- `Exclude<T, U>`, `Extract<T, U>`
- `NonNullable<T>`

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

## Modules

Module syntax matches JavaScript and TypeScript exactly.

### Imports

Imports work exactly like JavaScript/TypeScript.

#### Side-Effect Import

Import a module for its side effects only:

```
import "module";
```

#### Named Imports

Import specific exports by name:

```
import { foo, bar } from "module";
```

#### Aliased Import

Rename an import locally:

```
import { foo as f } from "module";
```

#### Namespace Import

Import all exports as a namespace object:

```
import * as mod from "module";

mod.foo();
mod.bar;
```

#### Default Import

Import the default export:

```
import Default from "module";
```

#### Combined Import

Import default and named together:

```
import Default, { foo, bar } from "module";
```

#### Type-Only Import

Import only types (erased at runtime):

```
import type { MyType } from "module";
import { type MyType, myValue } from "module";
```

### Exports

Exports work exactly like JavaScript/TypeScript.

#### Inline Export

Export declarations directly:

```
export const value = 42;
export function foo() { }
export struct Point { x: float32, y: float32 }
```

#### Named Exports

Export previously declared items:

```
const a = 1;
const b = 2;

export { a, b };
```

#### Aliased Export

Export with a different name:

```
export { internal as public };
export { foo as default };     // as default export
```

#### Default Export

Export a single default value:

```
export default function handler() { }
export default class MyClass { }
export default expression;
```

#### Re-Export

Forward exports from other modules:

```
export { foo, bar } from "module";    // specific items
export * from "module";               // all exports
export * as ns from "module";         // as namespace
```

#### Type-Only Export

Export only types (erased at runtime):

```
export type { MyType };
export { type MyType, myValue };
```

### Module Resolution

Destack uses the same module resolution as TypeScript/Node.
Destack also follows `tsconfig.json` configuration (incl. re-mapping).

### Data Imports

Destack supports importing non-code files with automatic type inference.

#### JSON, TOML, YAML

Data files are parsed and typed structurally:

```
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

```
import content from "./README.md";
// content: string
```

#### Binary Files

Binary files (images, fonts, wasm, etc.) import as `uint8[]`:

```
import data from "./image.png";
// data: uint8[]
```

#### Import Attributes

Override the default loader using import attributes:

```
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

```
import a from "./data.json";                           // parsed JSON
import b from "./data.json" with { type: "text" };    // raw string
// a and b are different modules
```

## Annotations

Destack has three kinds of annotations: comments, documentation, and decorators.
All annotations are preserved in the AST and available to tooling.
(The Destack AST is a superset of the TypeScript AST that also contains whitespace and concrete info like a traditional CST).

### Decorators

Decorators generalize TypeScript decorator semantics to enable decorators both as metadata and as transforms on most language constructs: declarations, statements, members, parameters, match arms, and more (not just classes and class members).

```
@memoize                           // decorator: memoize(target)
@route("/api/users")               // factory: route("/api/users")(target)
@service.middleware                // member access: service.middleware(target)
```

The decorator LHS-expression can be any expression (path, member access, call):
- `@foo` → calls `foo(target)`
- `@foo(args)` → calls `foo(args)(target)` (factory pattern)
- `@obj.method` → calls `obj.method(target)`

Decorators can be applied to most language constructs:

```
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

```
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

Standard JavaScript/TypeScript comments:

```
// line comment
/* block comment */
```

### Documentation

Documentation comments are attached to the following declaration and used for generated docs:

```
/// Line documentation comment.
/// Can span multiple lines.

/**
 * Block documentation comment.
 * Supports markdown formatting.
 */
```
