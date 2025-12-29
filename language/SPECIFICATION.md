# Destack Language Specification

The Destack language is "TypeScript++" for building optimal, correct, integrated full-stack software systems.
This document describes the syntax and semantics of **`.ds` files**.
`.ts` and `.js` files work exactly the same as before.
**Copy-pasting from `.js` or `.ts` into `.ds` works** for real-world code—see [Compatibility](DESIGN.md#compatibility) for rare edge cases.
See [DESIGN.md](DESIGN.md) for design and motivation.

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

String literals work exactly like in TypeScript, with the addition that single-quoted literals are recommended for single-character "strings":

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
[1, 2, 3]            // array
{ a: 1, b: 2 }       // anonymous struct/object
```

Destack adds tuple and typed struct literals:

```
(1, 2, 3)            // tuple
Point { x: 1, y: 2 } // typed struct literal
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

Destack extends TypeScript's type system with precise primitives, explicit reference semantics, types as values, static parameterisation of values, and some other goodies.

### Primitives

TypeScript has `number`, `string`, `boolean`, `bigint`, `symbol`, `null`, `undefined`, and `void`.
Destack adds precise numeric types while keeping the originals as aliases.

#### Special Types

All special types work like in TypeScript, with Destack extending the receiver-aware `this` type.

- `void` - empty type (no value)
- `null` - explicit zero/unset value
- `undefined` - uninitialized value (discouraged)
- `never` - bottom type (unreachable)
- `any` - erased top type (discouraged)
- `unknown` - explicit erased any (discouraged)
- `this` - receiver type (see below)

#### The `this` Type

Destack supports TypeScript's polymorphic `this` type for instance members, and extends it to static type positions.
`this` is only valid in type positions and resolves based on the surrounding declaration:

- In instance members, `this` resolves to the concrete receiver type.
- In static type positions (like static members or static arguments), `this` resolves to the containing type itself.

#### Booleans

Booleans work unchanged.

#### Numerics

TypeScript uses `number` for all numerics.
Destack supports that too, and adds more precise integer types:

- `int8`, `int16`, `int32`, `int64`, `int128` (signed)
- `uint8`, `uint16`, `uint32`, `uint64`, `uint128` (unsigned)
- `int` / `uint` - aliases for `int64` / `uint64`
- Arbitrary width integers: `int3`, `uint17`, etc.

Destack also supports specifying `float` explicitly:
- `float64` 
- `float` - alias for `float64`
- `number` - alias for `float64` (TypeScript compatibility)

Unlike TypeScript's single `number` type, Destack's precise integers behave like real machine integers:
they have defined overflow semantics (wrapping, saturating, or trapping), proper bitwise operations.
(Of course, when transpiled to )

#### Characters and Strings

In addition `string`, Destack supports a single `character`:
- `string` - UTF-8 string (same as TypeScript)
- `character` - single Unicode codepoint

### Type Aliases and Newtypes

Type aliases work like TypeScript, with Destack adding `newtype` for nominal (distinct) types:

```
type Point = { x: float32, y: float32 };   // structural (TypeScript)
newtype UserId = int64;                     // nominal (distinct type)
```

A `newtype` creates a distinct type—`UserId` and `OrderId` won't mix even if both are `int64`.
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

Pattern matching also works with newtype constructors:

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

Both approaches are compatible—the newtype erases at runtime to the underlying discriminated union, so you can still use structural matching like `{ kind: 'ok', value }` if preferred.

### Arrays and Tuples

Arrays work like TypeScript, with Destack adding fixed-length arrays and explicit tuple syntax:

```
int32[]              // dynamic array (TypeScript style)
int32[N]             // fixed-length array
(int32, boolean)     // tuple (explicit tuple)
(x: int32, y: int32) // named tuple
```

### References and Values

By default, any `T` behaves like in TypeScript (with value primitives and reference objects).
Destack additionally supports explicit ownership control:

```
T            // automatic (TypeScript behavior, implicitly GC-managed)
&T           // borrow (read-only reference)
&mut T       // borrow (mutable reference)
^T           // ownership transfer (caller gives up ownership)
^mut T       // ownership transfer (explicitly mutable)
```

| Modifier | Meaning | After `foo(x)` | Who cleans up? |
|----------|---------|----------------|----------------|
| `T` | GC-managed (implicit) | `x` still valid | GC |
| `&T` | Borrow (read) | `x` still valid | Original owner |
| `&mut T` | Borrow (mutate) | `x` still valid, maybe changed | Original owner |
| `^T` | Ownership transfer | `x` **invalid** | New owner (or GC fallback) |

**Use-after-move:**

```
const node = AstNode { ... }
consume(^node)    // ownership transferred
print(node.value) // ERROR: use after ownership transfer
```

Using a value after ownership transfer is an error (suppressible to warning).

**Drop as soon as possible:**

`^T` values are dropped at their last proven use (non-lexical).
The compiler inserts a drop as soon as it can prove the value is no longer needed,
even if the lexical scope continues.

```
function process() {
    const data = ^LargeData { ... }  // we own this
    doWork(&data)                     // borrow it
    log("done")                       // data can be dropped before this line
}
```

Types can implement `Drop` to customize cleanup, enabling RAII patterns.

Strings follow the same ownership spectrum: `string` is GC-managed by default,
`&string` is a borrowed view, and `^string` is explicitly owned.

**Returning references:**

Functions can return `&T`. 
The compiler warns on obvious mistakes (returning reference to local), but does not enforce full lifetime tracking.

```
function get(c: &Container): &Item { &c.item }  // ok

function bad(): &Point {
    const p = Point { x: 1, y: 2 }
    &p  // WARNING: returning reference to local
}
```

### Dynamic Parameterisation

Functions and methods work exactly like in JavaScript and TypeScript.
Function calls use positional arguments:

```
function myFunction(a: int, b: int) {
    ...
}

myFunction(2, 3);
```

### Static Parameterisation ("Generics")

Static parameterisation for types works like in TypeScript.
In Destack, static parameters also work for "compile-time" values and look more like dynamic parameters (though TypeScript syntax with `T extends U` is still supported).

```
struct Container<T: any> { // works like T extends any
    value: T
}

function identity<T>(x: T): T {
    x
}
```

Non-type parameters for "compile-time" constants:
```
function compute<Foo: boolean>(data: uint8[]) {
    if Foo {
        ...
    }

    ...
}

compute<true>(); // pass the static argument positionally
```

### Where Clauses

Destack adds `where` clauses for type constraints beyond TypeScript's inline syntax:

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

### Refinements

Refinements add constraints and metadata to types.
The compiler checks provided refinements at compile time where provable; runtime validation is opt-in via the `@destack-sh/schema` library.

```
int.min(0)                           // int >= 0
int.max(100)                         // int <= 100  
int.min(0).max(100)                  // int in [0, 100]
string.minLength(1)                  // non-empty string
string.tag("myTag")      // with metadata
uint[].nonEmpty()                    // non-empty array
```

Refinements are implemented as extensions, so user can define domain-specific ones.
That is exactly how the `@destack-sh/schema` ones work (there is no privileged magic here).

#### Standard Library Refinements

Refinements are defined via extensions on types (just like any `extension` on any type).
The standard library bundles many useful refinements commonly used in schema libraries: 

**Numeric types** (`int`, `uint`, `float`, and sized variants):

| Refinement | Meaning |
|------------|---------|
| `.min(n)` | Value ≥ n |
| `.max(n)` | Value ≤ n |
| `.positive()` | Value > 0 |
| `.negative()` | Value < 0 |
| `.nonZero()` | Value ≠ 0 |

**String type**:

| Refinement | Meaning |
|------------|---------|
| `.minLength(n)` | Length ≥ n |
| `.maxLength(n)` | Length ≤ n |
| `.nonEmpty()` | Length ≥ 1 |
| `.length(n)` | Length = n |

**Array-like types**:

| Refinement | Meaning |
|------------|---------|
| `.minLength(n)` | Length ≥ n |
| `.maxLength(n)` | Length ≤ n |
| `.nonEmpty()` | Length ≥ 1 |

#### Composing Refinements

Types are values, refinements refine those values, and refined types can be composed:
```
// variables
const percentage: float.min(0).max(100) = 75.5;

// functions
function clamp(x: int, min: int, max: int): int.min(min).max(max);

// composites
struct User {
    name: string.minLength(1).maxLength(100),
    age: uint.max(150),
}

// aliases
type Percentage = float.min(0).max(100);
type NonEmptyString = string.nonEmpty();
```

Refined types propagate through assignments and narrowing:
```
function process(x: int.min(0).max(100)) {
    // x is known to be in [0, 100]
    
    if x > 50 {
        // x is narrowed to int.min(51).max(100)
    }
}

const a: int.min(10) = 15;
const b: int.min(0) = a;    // ok: min(10) implies min(0)
const c: int.min(20) = a;   // error: min(10) doesn't imply min(20)
```

#### Refinement Validation

The compiler checks refinements at compile time when values are provable:

```
function setAge(age: uint.max(150)) { }

setAge(30);      // ok: 30 ≤ 150
setAge(200);     // compile error: 200 > 150

const x = 25;
setAge(x);       // ok: x is known to be 25

let y = getInput();
setAge(y);       // compile error: can't prove y ≤ 150
```

When the compiler can't prove a refinement, it's a **compile error**, and you need to coerce or dynamically check.
For the standard library `@destack-sh/schema`, we provide convenient checks:

```
import { parse, safeParse } from "@destack-sh/schema";

let y = getInput();
const validated = parse(uint.max(150), y);  // throws if y > 150
setAge(validated);                           // ok: validated has refined type

// or without throwing:
const result = safeParse(uint.max(150), y);
if (result.ok) {
    setAge(result.value);
}
```

## Comptime

Inspired by Zig, Destack supports compile-time evaluation via the `comptime` keyword.
Unlike Zig or Rust macros, however, Destack's comptime fills in well-defined **typed slots** rather than enabling fully arbitrary code generation.
In practice, this `comptime` behavior and specialisation together with decorators enable most macro-style use cases without the unpredictability and compiler complexity of a "full" macro system.

### Static Execution vs Comptime Execution

Destack uses two related but separate "evaluate during compile time" mechanisms:

- **Static execution** is evaluated during Analyze to produce `StaticExpression` values.
  This is a restricted subset of expressions that can be folded without executing user code.
  Static execution is required for static parameters, array sizes, and other type-driven
  constructs that must be known for analysis (i.e., type checking).
- **Comptime execution** evaluates `comptime` expressions and blocks during the Execute phase
  by running MIR in the Machine interpreter. The result is substituted back into the program
  as a constant and any comptime-controlled branches are eliminated.

Static execution must not depend on full comptime execution. This avoids dependency cycles
between type resolution and code evaluation.

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
Note here that the result type must be specified upfront.

The result is embedded as a constant in the compiled output.

### Comptime Blocks

Comptime blocks can appear as struct/class members or at module top-level.
They run during compilation rather than at runtime.

#### Member Comptime Blocks

Comptime blocks inside structs or classes run once per instantiation of the type.
They are useful for compile-time assertions on static parameters:

```
struct Buffer<size: uint> {
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

Static parameters ("generics" with support for non-type values) are inherently "comptime":

```
function repeat<N: int>(value: string): string {
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

### Comptime Parameters

Parameters can be marked `comptime` to require compile-time-known arguments:

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

Comptime parameters enable:
- Specialization based on constant values
- Compile-time loop unrolling
- Dead code elimination based on parameter values

Comptime parameters still require **static expressions** as arguments. The call site may
evaluate the function body via comptime execution, but the arguments themselves must be
known during Analyze.

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

Destack's comptime is designed around the concept of **typed slots**:

1. **Types are fixed during Analyze**: all bindings, symbols, and types are determined.
2. **Comptime expressions are slots**: positions where a value will be computed.
3. **Execute phase fills the slots**: computes values, eliminates dead branches.
4. **No binding changes**: the symbol table never changes after Analyze.

#### Code Specialization

Comptime cannot not generate arbitrary code (no `eval`!).
But, we can specialize code statically to cover essentially all relevant use cases via comptime specialization, which is more maintainable and understandable anyway.
In practice, this isn't its own "feature", but just a nice consequence of other orthogonal features:

1. **Monomorphization**: generic functions become specialized per type argument.
2. **Loop unrolling**: iteration over comptime known collections (like `Type<T>.fields`) is unrolled.
3. **Branch elimination**: comptime conditionals select which code survives.
4. **Inlining**: comptime expressions become constants.

#### Comptime Example: JSON Parser

This example demonstrates code specialization by creating a type safe JSON parser.

```ds
@inline
function parse<T>(json: string): T {
    const obj = JSON.parse(json);
    let result: Partial<T> = {};

    @unroll
    for (const field of comptime Type<T>.fields) {
        if (comptime field.type extends string) {
            result[field.name] = String(obj[field.name]);
        } else if (comptime field.type extends number) {
            result[field.name] = Number(obj[field.name]);
        } else if (comptime field.type extends boolean) {
            result[field.name] = Boolean(obj[field.name]);
        } else {
            result[field.name] = parse<typeof field.type>(obj[field.name]);
        }
    }

    return result as T;
}

struct User { 
    name: string, 
    age: int, 
    active: boolean 
}

// when instantiated, the loop is unrolled and branches are eliminated:
const user = parse<User>('{"name": "Alice", "age": 30, "active": true}');
```

After comptime evaluation with `User`, the function becomes:

```ds
function parse_User(json: string): User {
    const obj = JSON.parse(json);
    let result: Partial<User> = {};

    result.name = String(obj.name);
    result.age = Number(obj.age);
    result.active = Boolean(obj.active);

    return result as User;
}
```

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
    bucketCount: uint16,
    bucketSize: uint16,
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

## Reflection

Destack makes types first-class runtime values, enabling reflection without separate metadata systems or configuration.
This section specifies the built-in reflection capabilities; the standard library `@destack-sh/schema` extends these with validation utilities.

### Type Descriptors

Every type `T` has a corresponding runtime value of type `Type<T>`.
Using a type name in value position evaluates to its descriptor:

```
struct Point { x: float32, y: float32 }

const t = Point;              // t: Type<Point>
const t: Type<Point> = Point; // explicit annotation
```

The `typeOf` function returns a descriptor for a value's type (unlike `typeof`, which returns a coarse-grained string like `"object"`):

```
const p = Point { x: 1, y: 2 };
const t = typeOf(p);          // Type<Point>
```

### Runtime Type Identity

Runtime type identity (RTTI) is demand-driven. The compiler only emits RTTI for types that
are used at runtime (e.g., `typeOf`, `instanceof`, `any`/`unknown`, or runtime reflection).
Classes always carry a vtable pointer for dynamic dispatch and RTTI. Structs are pure data
unless RTTI is required by usage. On JS targets, RTTI-enabled values use a hidden symbol
property rather than a global WeakMap, preserving "plain object" semantics.

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
const x: Drawable = { draw() {} }  // OK: structural match

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

Classes work like TypeScript—reference types with identity and prototype-based inheritance:

```
class MyClass {
    field: int32

    constructor(value: int32) {
        this.field = value
    }
}
```

Classes have **identity**: two instances are only `===` if they're the same object:

```
const a = new MyClass(1);
const b = new MyClass(1);
a == b    // false: different instances (unless Equal implemented)
a === b   // false: different instances
a === a   // true: same instance
```

This is the key difference from structs—see the comparison table below.

### Struct

Destack adds `struct` for nominal object types with fixed layout.
Structs are simpler than classes: no reference identity, no inheritance, just data with a name.

```
struct Point {
    x: float32
    y: float32
}
```

#### Struct vs Class

| | struct | class |
|---|---|---|
| Reference identity | ❌ No (`===` is error) | ✅ Yes (`===` compares pointers) |
| Inheritance | ❌ No (use embedding) | ✅ Yes (`extends`) |
| Default passing | Reference | Reference |
| JS output | Plain object | ES6 class |

#### Equality and Identity

Structs have no reference identity—two structs with the same properties are equal by value.
Structs auto-derive `Equal` (field-by-field comparison) by default:

```
const p1 = Point { x: 1, y: 2 };
const p2 = Point { x: 1, y: 2 };
p1 == p2   // true: same fields = equal (auto-derived Equal)
p1 === p2  // error: === requires reference identity, structs have none
```

Since structs have no reference identity, `===` and `!==` are compile errors on struct types.
Use `==` for value comparison.

#### Construction

Structs are nominal, so they must be explicitly constructed:

```
let p: Point = Point { x: 1, y: 2 }  // ok: explicit construction
let p: Point = new Point(1, 2)       // ok: constructor syntax
let p: Point = { x: 1, y: 2 }        // error: object literal is not Point
```

#### Pattern Matching

Struct patterns require the type name (unlike newtypes which auto-unwrap):

```
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at ${x}, ${y}`
    { x, y } => ...  // error: structural pattern on nominal type
}
```

#### Interfaces and Composition

Structs can `implements` interfaces but cannot `extends` (use embedding instead):

```
struct Point implements Drawable {
    x: float32
    y: float32
    draw(): void { ... }
}

struct Transform { position: Vec3, rotation: Quat }
struct Player {
    ...Transform    // embeds Transform's fields (composition)
    health: int
}
```

At runtime in JS, a struct is just a plain object—its nominal type is erased.
Ownership (`&T`, `^T`) is orthogonal: you can explicitly reference or copy either structs or classes.

#### Associated Types

Structs and classes can declare associated type aliases using the `type` keyword:

```
struct Container<T> {
    type Item = T;
    type Iter = ContainerIterator<T>;

    items: T[],

    iter(): Iter { ContainerIterator { items: this.items } }
}
```

Associated types are inherently static because they belong to the type itself, not to instances.
Associated types are resolved at compile time based on the type's static parameters.

Associated types can be accessed via the containing type:

```
const item: Container<int>.Item = 42;  // Item resolves to int
```

In interfaces, associated types can be declared without a default (abstract) or with a default:

```
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

Enums work like TypeScript:

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
For union argument types, each union member is matched against the overloads:

```
function handle(x: string): string;
function handle(x: number): number;

const y: string | number = getValue();
// both overloads may be called at runtime
// returns: `string | number`
handle(y);  
```

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
    name: string
    
    constructor(name: string) {
        this.name = name
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
Unlike TypeScript, `if` is an expression that returns a value:

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

TypeScript's `switch` also works unchanged:

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

#### Result and the ? Operator

The `?` operator propagates `Result` errors to the caller:

```
function readConfig(path: string): Result<Config, Error> {
    const text = readFile(path)?    // returns early if Err
    const json = parseJson(text)?   // returns early if Err
    Result.ok(Config.from(json))
}
```

When `?` is applied to a `Result<T, E>`:
- If `Ok(value)`, extracts and returns `value`
- If `Err(e)`, returns early with `Err(e)` from the enclosing function

The enclosing function must have a compatible `Result` return type.

#### Result and the ?? Operator

The `??` operator extracts the success value or uses a default:

```
const config = loadConfig() ?? defaultConfig;
const port = parsePort(input) ?? 8080;
```

When `??` is applied to a `Result<T, E>`:
- If `Ok(value)`, extracts and returns `value`
- If `Err(_)`, returns the right-hand default value

Both `?` and `??` work via the `Try` interface. For nullable types (`T | null`),
`??` behaves exactly like TypeScript's nullish coalescing operator.

#### try/catch on Result

The `try`/`catch` syntax works with `Result` types as pattern matching sugar:

```
try {
    const config = readConfig("config.json")?
    process(config)
} catch (e: IOError) {
    log("Failed:", e)
} finally {
    cleanup()
}
```

This desugars to a `match` on the `Result`. No stack unwinding is involved.

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

### Type Operators

Type-level operators (not overloadable):

| Operator | Description | Interface |
|----------|-------------|-----------|
| `as` | Type cast | — |
| `is` | Type guard | — |
| `instanceof` | Instance check | — |
| `satisfies` | Type satisfaction | — |
| `typeof` | Get type | — |
| `keyof` | Get keys | — |
| `extends` | Subtype check | — |
| `implements` | Interface check | — |

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

@unroll                    // hint to compiler, stripped in JS output
for (let i = 0; i < 4; i++) { }

@deprecated("use newAPI")  // compile-time warning, stripped in JS output
function oldAPI() { }
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
