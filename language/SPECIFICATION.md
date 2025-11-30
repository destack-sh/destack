# Destack Language

Destack is TypeScript++ for building better full-stack software systems.
This document describes the syntax and semantics of **`.ds` files**.
`.ts` and `.js` files work exactly the same as before. 
Even **copy-pasting from `.js` or `.ts` into `.ds` works**: 

**Valid JavaScript is valid Destack.
Valid TypeScript is valid Destack.**

Where Destack looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it *is* TypeScript++.
Unlike in `C++`, our `C` (JS/TS) still works perfectly in Destack, and all `++` features are opt-in and complementary:

- **Expressions as values**: `if`, `match`, blocks are values
- **Types as values**: type annotations *are* values (available at runtime)
- **Decorators and tags**: richer annotations for metaprogramming (available at runtime) 
- **Precise primitives**: `int32`, `float64` instead of just `number`
- **Pattern matching**: patterns with `match` expressions
- **Mutability and references**: explicit `&T`, `^T` and `&mut T` 
- **Value types and structs**: value-oriented data types with `struct`
- **Constraints**: more precise types with `where` guards 
- **Effects**: context declaration and management `with` clauses
- **Extensions**: extend types and organize complex implementations
- **Operator overloading**: opt-in via extensions and interfaces
- **Function overloading**: opt-in function overloading without clumsy types

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

### Raw Strings

Destack adds raw strings disable escape sequence processing (like Rust):

```
r#"raw string with "quotes" and \n literally"#
r##"can use multiple # for nesting"##
```

### Byte Literals

Destack adds byte literals for working with raw bytes:

```
b'a'                 // byte (u8)
b"abc"               // byte string (sequence of u8)
```

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

Tree literals use TSX-like syntax for hierarchical data structures.
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

The tree literal syntax is customizable via traits, so your domain types can define how they're constructed from tree syntax.

## Types

Destack extends TypeScript's type system with precise primitives, explicit reference semantics, types as values, static parameterisation of values, and some other goodies.

### Primitives

TypeScript has `number`, `string`, `boolean`, `bigint`, `symbol`, `null`, `undefined`, and `void`.
Destack adds precise numeric types while keeping the originals as aliases.

#### Special Types

All special types work like in TypeScript:

- `void` - empty type (no value)
- `null` - explicit zero/unset value
- `undefined` - uninitialized value (discouraged)
- `never` - bottom type (unreachable)
- `any` - erased top type (discouraged)
- `unknown` - explicit erased any (discouraged)

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
type Point = { x: float32, y: float32 }   // structural (TypeScript)
newtype UserId = int64                     // nominal (distinct type)
```

A `newtype` creates a distinct type—`UserId` and `OrderId` won't mix even if both are `int64`.
Newtypes can have methods via extensions (see Extensions below).
The same mechanism works for all types: structs, enums, newtypes, even foreign types and builtin primitives like `int32` or `Date`.

### Unions and Intersections

Combinator types work like in TypeScript:

```
int32 | string | null      // union
A & B                      // intersection
```

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
Destack additionally supports explicit value or reference control for `T`:

```
T            // automatic (TypeScript behavior)
&T           // immutable reference
&var T       // mutable reference
^T           // value (copy semantics)
^var T       // mutable value
```

### Dynamic Parameterisation

Functions and methods work exactly like in JavaScript and TypeScript.
In Destack, you can also pass named function arguments:

```
function myFunction(a: int, b: int) {
    ...
}

myFunction(a: 2, b: 3)
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

compute<Foo: true> // static parameters may also be named
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

## Context and Effects

Destack adds effect tracking to declare and manage what a function can do.

### With Clauses

`with` clauses declare which effects or contexts a function requires:

```
function readFile(path: string): string with FileSystem {
    // ...
}

function pure<T>(x: T): T with !Allocation {
    // must not allocate
}
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
    static Empty: Self
    size(): uint64
    get(index: uint64): T?
}
```

### Class

Classes work like TypeScript:

```
class MyClass {
    field: int32
    
    constructor(value: int32) {
        this.field = value
    }
}
```

### Struct

Destack adds `struct` for data-oriented types with value semantics and no constructor.
Structs may be embedded in other types (like structs or classes).

```
struct Point {
    x: float32
    y: float32
}

class MyEntity extends Entity {
    id: uint64
    ...Transform           // embed Transform properties and methods
}
```

Unlike classes, structs are passed by value (i.e., copied) by default.
And also unlike classes, a struct at runtime is just another "object" - its type disappears. 

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
        match this {
            Active => "Active"
            Inactive => "Inactive"
            Pending => "Pending..."
        }
    }
}
```

### Extension

Extensions add methods to existing types without modifying them.
Unlike TypeScript's prototype extension, Destack extensions are type-safe and scoped.

#### Basic Extensions

Add methods to any type—structs, enums, newtypes, or primitives:

```
extension Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }
    
    normalized(): Vector2 {
        const m = this.magnitude()
        Vector2 { x: this.x / m, y: this.y / m }
    }
}
```

#### Extending Newtypes and Primitives

The same mechanism works for newtypes and even precise primitives:

```
newtype UserId = int;
extension UserId {
    isValid(): boolean { this > 0 }
}

// anonymous extension to foreign type (only reachable in same scope)
extension int32 {
    abs(): int32 { if this < 0 { -this } else { this } }
}
```

#### Implementing Interfaces

Extensions can implement interfaces, enabling operator overloading:

```
extension Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}

// now you can use: v1 + v2
```

#### Foreign Extensions

Extend types from other modules:

```
import { Vector2 } from "somewhere";

extension Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }
}
```

#### Named Extensions

For explicit scoping, extensions can be named and must be imported:

```
import { Date } from "builtin";

export extension DateUtils: Date implements Add<Date> {
    addDays(days: int): Date { ... }
}
```

Then import explicitly:

```
import { DateUtils } from "./utilities";
// now Date has .addDays()
```

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
    const response = await fetch(url)
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
    let (a, b) = (0, 1)
    loop {
        yield a
        (a, b) = (b, a + b)
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
const v = Vector2 { x: 3, y: 4 }
v.magnitude()        // 5.0
Vector2.zero()       // static call
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

sum(1, 2, 3, 4, 5)

const args = [1, 2, 3]
sum(...args)
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
const result = if x > 0 { "positive" } else { "negative" }
const label = match state {
    Ready => "go"
    Loading => "wait"
}
```

### Bindings

Variable bindings work like TypeScript, with Destack adding tuple destructuring syntax:

```
const x = 1              // immutable
const x: int32 = 1       // with type
let y = 2                // mutable

const [a, b] = getTuple()
const { x, y } = getPoint()
const (a, _) = getTuple()  // Destack tuple syntax
```

Unlike JavaScript/TypeScript, bindings can be re-declared in the same scope with a different type (like Rust):

```
const x = "hello"        // x is string
const x = x.length       // x is now int (shadowing)
```

### Blocks

Block expressions group multiple statements and return the last expression's value.
Blocks are enclosed in `{ }` and can optionally have labels:

```
const result = {
    const x = compute()
    const y = transform(x)
    x + y                    // last expression is the block's value
}
```

Labeled blocks allow breaking with values:

```
const value = outer: {
    for i in 0..100 {
        if condition(i) {
            break outer i    // return i from the block
        }
    }
    -1                       // default if no break
}
```

For disambiguation (e.g., after `if` or `match`), use `do`:

```
const x = if flag { do { compute() } } else { 0 }
```

### Conditionals

Control flow with `if` works like TypeScript, with optional parentheses.
Unlike TypeScript, `if` is an expression that returns a value:

```
// statement form (both valid)
if (x > 0) { process() }
if x > 0 { process() }

// if/else
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// as expression - returns a value
const sign = if x > 0 { 1 } else if x < 0 { -1 } else { 0 }
const message = if ready { "go" } else { "wait" }

// ternary (same as TypeScript)
const sign = x > 0 ? 1 : x < 0 ? -1 : 0
```

### Match

Destack adds `match` expressions for exhaustive pattern matching.
Like `if`, `match` is an expression that returns a value:

```
// match as expression - returns the matched arm's value
const label = match state {
    Ready => "go"
    Loading => "wait"
    Error(e) => `failed: ${e}`
}

// match on values
match value {
    0 => "zero"
    1 | 2 | 3 => "small"
    n if n < 0 => "negative"
    _ => "other"
}

// match with destructuring
match point {
    (0, 0) => "origin"
    (x, 0) => `x-axis at ${x}`
    (0, y) => `y-axis at ${y}`
    (x, y) => `at (${x}, ${y})`
}

// match with guards
match user {
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

Destack supports all TypeScript loop forms plus `for-in` and `loop`.

#### For-In

Iterate over iterables with `for-in`, which works with arrays, ranges, and any iterable:

```
for item in items {
    process(item)
}

// with range literals
for i in 0..10 {
    print(i)          // 0, 1, 2, ..., 9
}

for i in 0..=10 {
    print(i)          // 0, 1, 2, ..., 10 (inclusive)
}

// with destructuring
for (key, value) in map {
    print(`${key}: ${value}`)
}
```

#### While

Standard while loops work like TypeScript:

```
while condition {
    process()
}

// do-while
do {
    process()
} while condition
```

#### Loop

Infinite loop that can be exited only with `break`:

```
loop {
    const input = readInput()
    if input == "quit" {
        break
    }
    process(input)
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
outer: for i in 0..10 {
    for j in 0..10 {
        if condition {
            break outer      // exits outer loop
        }
        if other {
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
var x                // bind to mutable x
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

#### Struct

Matches struct/object fields:

```
{ x, y }             // shorthand
{ x: a, y: b }       // rename bindings
{ x, ...rest }       // rest pattern
Point { x: 0, y }    // typed with literal field
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

TypeScript's `try`/`catch` works unchanged:

```
try {
    riskyOperation()
} catch (e) {
    handleError(e)
} finally {
    cleanup()
}
```

Destack adds propagation operators:

```
const result = riskyOperation()?;   // propagate to next outer scope
```

### Defer

`defer` schedules an expression to run when the current scope exits, regardless of how it exits (normal return, early return, or error).
This is useful for cleanup or other logic that must happen in any case:

```
defer file.close()                 // single expression

defer {                            // block form
    cleanup()
    log("done")
}
```

Deferred expressions run in reverse order of declaration (LIFO), so resources are cleaned up in the opposite order they were acquired.

## Operators

Destack operators match TypeScript with additional precision for integer arithmetic.
All operators are overloadable via extensions implementing the corresponding interface (e.g., `Add`, `Subtract`, `Multiply`).

### Arithmetic

Standard arithmetic operators work like TypeScript:

| Op | Description | Assignment |
|----|-------------|------------|
| `+` | Add | `+=` |
| `-` | Subtract | `-=` |
| `*` | Multiply | `*=` |
| `/` | Divide | `/=` |
| `%` | Remainder | `%=` |

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
const a: uint8 = 250
const b: uint8 = 10

a + b          // default overflow behavior
a +% b         // wrapping: 250 + 10 = 4 (wraps around 256)
a +| b         // saturating: 250 + 10 = 255 (clamped to max)
```

### Comparison

Comparison operators work like TypeScript:

| Op | Description |
|----|-------------|
| `==` | Equal |
| `!=` | Not equal |
| `===` | Strict equal |
| `!==` | Strict not equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |

### Logical

Logical operators work like TypeScript:

| Op | Description |
|----|-------------|
| `&&` | Logical and |
| `\|\|` | Logical or |
| `??` | Nullish coalescing |
| `!` | Logical not |

### Bitwise

Bitwise operators work like TypeScript:

| Op | Description | Assignment |
|----|-------------|------------|
| `&` | Bitwise and | `&=` |
| `\|` | Bitwise or | `\|=` |
| `^` | Bitwise xor | `^=` |
| `~` | Bitwise not | |
| `<<` | Shift left | `<<=` |
| `>>` | Shift right | `>>=` |
| `>>>` | Unsigned shift right | `>>>=` |

Destack adds saturating shift:

| Op | Description | Assignment |
|----|-------------|------------|
| `<<\|` | Saturating shift left | `<<\|=` |

### Type Operators

Destack supports TypeScript's type operators and adds more:

| Op | Description |
|----|-------------|
| `as` | Type cast |
| `is` | Type guard |
| `instanceof` | Instance check |
| `satisfies` | Type satisfaction |
| `typeof` | Get type |
| `keyof` | Get keys |
| `extends` | Subtype check |
| `implements` | Interface check |

### Other Operators

Additional operators:

| Op | Description |
|----|-------------|
| `?` | Optional chaining / unwrap |
| `!` | Force unwrap (non-null assertion) |
| `?.` | Optional member access |
| `??` | Nullish coalescing |
| `...` | Spread / rest |
| `++` / `--` | Increment / decrement |

## Modules

Module syntax matches JavaScript and TypeScript exactly.

### Imports

Imports work exactly like JavaScript/TypeScript.

#### Side-Effect Import

Import a module for its side effects only:

```
import "module"
```

#### Named Imports

Import specific exports by name:

```
import { foo, bar } from "module"
```

#### Aliased Import

Rename an import locally:

```
import { foo as f } from "module"
```

#### Namespace Import

Import all exports as a namespace object:

```
import * as mod from "module"

mod.foo()
mod.bar
```

#### Default Import

Import the default export:

```
import Default from "module"
```

#### Combined Import

Import default and named together:

```
import Default, { foo, bar } from "module"
```

#### Type-Only Import

Import only types (erased at runtime):

```
import type { MyType } from "module"
import { type MyType, myValue } from "module"
```

### Exports

Exports work exactly like JavaScript/TypeScript.

#### Inline Export

Export declarations directly:

```
export const value = 42
export function foo() { }
export struct Point { x: float32, y: float32 }
```

#### Named Exports

Export previously declared items:

```
const a = 1
const b = 2

export { a, b }
```

#### Aliased Export

Export with a different name:

```
export { internal as public }
export { foo as default }     // as default export
```

#### Default Export

Export a single default value:

```
export default function handler() { }
export default class MyClass { }
export default expression
```

#### Re-Export

Forward exports from other modules:

```
export { foo, bar } from "module"    // specific items
export * from "module"               // all exports
export * as ns from "module"         // as namespace
```

#### Type-Only Export

Export only types (erased at runtime):

```
export type { MyType }
export { type MyType, myValue }
```

### Module Resolution

Destack uses the same module resolution as TypeScript/Node.
Destack also follows `tsconfig.json` configuration (incl. re-mapping).

## Annotations

Destack has four kinds of annotations: comments, documentation, tags, and decorators.
All annotations are preserved in the AST and available to tooling.
(The Destack AST is a superset of the TypeScript AST but also contains whitespace and concrete info like a traditional CST).

### Tags

Tags are structured metadata attached to any node.
Tags can receive arguments in optional parentheses (just like methods), which may also be overloaded. 
Everything is type-checked and queryable by tooling:

```
#deprecated                        // simple tag
#Performance                       // categorization
#todo(priority: high)              // tag with arguments
#version(1, 2, 3)                  // version annotation
```

Tags can appear in comments for compatibility with existing conventions:

```
// NOTE #Performance: this could be optimized
// TODO #Cleanup: refactor this
```

### Decorators

Decorators are "compile-time" transformations applied to declarations.
Like tags, decorators can receive arguments (in optional parentheses).
Decorators must be applied to "block-scoped" expressions like functions, properties or types.

```
@memoize                           // caches function results
@deprecated("use newFunction")     // deprecation with message
@entity                            // marks an entity type
@route("/api/users")
```

Unlike tags (which are passive metadata), decorators actively transform or augment the decorated declaration.

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