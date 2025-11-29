# Destack Language Specification

Destack is TypeScript extended for building better full-stack systems.
This document describes the syntax and semantics of `.ds` files.

**Valid JavaScript is valid Destack.
Valid TypeScript is valid Destack.**

When something looks like TypeScript (e.g., `interface`, `class`, `async`/`await`), it behaves like TypeScript, because it *is* TypeScript.
Everything below describes what Destack *adds* to TypeScript to enable higher correctness and better ergonomics.

## Key Extensions to TypeScript

The highlight reel - the ++ in Destack's "TypeScript++":

- **Expressions as values**: `if`, `match`, blocks are values
- **Types as values**: type annotations *are* values available at runtime
- **Precise primitives**: `int32`, `float64` instead of just `number`
- **Precise mutability**: `&`, `^` and 
- **Pattern matching**: exhaustive `match` expressions
- **Structs**: value-oriented data types
- **Constraints and effects**: `where` and `with` clauses
- **Extensions**: add methods to existing types and organize code
- **Operator overloading**: opt-in via extensions and interfaces
- **Function overloading**: opt-in function overloading without clumsy types
- **Optional parentheses**: `if x > 0 { }` instead of `if (x > 0) { }`

## Type System

Destack extends TypeScript's type system with precise primitives, explicit reference semantics and some extra expressive power.

### Primitives

TypeScript has `number`, `string`, `boolean`, `bigint`, `symbol`, `null`, `undefined`, and `void`.
Destack adds precise numeric types while keeping the originals as aliases.

#### Special Types

These work like TypeScript:

- `void` - empty type (no value)
- `null` - explicit zero/unset value
- `undefined` - uninitialized value
- `never` - bottom type (unreachable)
- `any` - top type (escape hatch)
- `unknown` - safe top type
- `boolean` - true or false

#### Integers

TypeScript uses `number` for all numerics.
Destack adds precise integer types:

- `int8`, `int16`, `int32`, `int64`, `int128` (signed)
- `uint8`, `uint16`, `uint32`, `uint64`, `uint128` (unsigned)
- `int` / `uint` - aliases for `int64` / `uint64`
- Arbitrary width: `int3`, `uint17`, etc.

#### Floats

- `float16`, `float32`, `float64`, `float128`
- `float` - alias for `float64`
- `number` - alias for `float64` (TypeScript compatibility)

#### Characters and Strings

- `char` - single Unicode codepoint
- `string` - UTF-8 string (same as TypeScript)

### Composite Types

#### Arrays and Tuples

```
int32[]              // dynamic array (TypeScript style)
int32[N]             // fixed-length array
[int32, boolean]     // tuple (TypeScript style)
(int32, boolean)     // tuple (Destack alternative)
(x: int32, y: int32) // named tuple
```

#### Structs

Structs are data-oriented types with value semantics:

```
struct Vector2 {
    x: float32
    y: float32
}

{ a: int32, b: boolean }   // anonymous struct
```

#### Unions and Intersections

These work like TypeScript:

```
int32 | string | null      // union
A & B                      // intersection
```

### References and Values

By default, Destack behaves like TypeScript.
For explicit control:

```
T            // automatic (TypeScript behavior)
&T           // immutable reference
&var T       // mutable reference
^T           // value (copy semantics)
^var T       // mutable value
```

### Type Aliases and Newtypes

```
type Point = { x: float32, y: float32 }   // structural (TypeScript)
newtype UserId = int64                     // nominal (distinct type)
```

A `newtype` creates a distinct type—`UserId` and `OrderId` won't mix even if both are `int64`.

Newtypes can have methods via extensions (see Extensions below).
The same mechanism works for all types: structs, enums, newtypes, even precise primitives like `int32` or `Date`.

### Generics

Generics work like TypeScript:

```
struct Container<T> {
    value: T
}

function identity<T>(x: T): T {
    x
}
```

Non-type parameters for compile-time constants:

```
function compute<Validate: boolean>(data: uint8[]) {
    // Validate is a compile-time constant
}
```

## Constraints and Effects

Destack adds `where` for type constraints and `with` for effect declarations.

### Where Clauses

Type constraints beyond TypeScript's inline syntax:

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

### With Clauses

Declare what effects a function requires:

```
function readFile(path: string): string with FileSystem {
    // ...
}

function pure<T>(x: T): T with !Allocation {
    // must not allocate
}
```

## Declarations

### Struct

Data-oriented types with value semantics:

```
struct Point {
    x: float32
    y: float32
}

struct Entity<T> extends Base {
    id: uint64
    data: T
    ...Transform           // embed Transform properties
    
    static Zero = Entity { id: 0, data: null }
    
    getId(): uint64 {
        this.id
    }
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

### Enum

Enums work like TypeScript, but can have methods via extensions:

```
enum Status {
    Active
    Inactive
    Pending
}

enum Priority {
    Low = 1
    Medium = 2
    High = 3
}

// add methods to enums (same mechanism as structs, newtypes, etc.)
extension Status {
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

This uses the same `extension` mechanism that works for all types.

### Interface

Interfaces work like TypeScript, with optional default implementations:

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

### Extension

Add methods to *any* type—structs, enums, newtypes, even primitives:

```
extension Vector2 {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }
}

extension Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}

// works for newtypes
extension UserId {
    isValid(): boolean { this > 0 }
}

// works for precise primitives
extension int32 {
    abs(): int32 { if this < 0 { -this } else { this } }
}
```

This is a unified mechanism: the same `extension` syntax adds methods to structs, enums, newtypes, and primitives.
Operator overloading works the same way—`a + b` calls `a.add(b)` if `Add` is implemented (and errors in obvious ways if not!).

### Function

Functions work like TypeScript:

```
function greet(name: string): string {
    `Hello, ${name}!`
}

async function fetchData(url: string): Promise<Response> {
    // ...
}

function* range(start: int32, end: int32): Generator<int32> {
    for i in start..end {
        yield i
    }
}
```

Lambdas:

```
(x) => x * 2
(a: int32, b: int32): int32 => a + b
```

### Namespace

Namespaces work like TypeScript:

```
namespace math {
    export const PI = 3.14159
    export function sin(x: float64): float64 { /* ... */ }
}
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

```
const x = 1              // immutable
const x: int32 = 1       // with type
let y = 2                // mutable

const [a, b] = getTuple()
const { x, y } = getPoint()
const (a, _) = getTuple()  // Destack tuple syntax
```

### Control Flow

Control flow works like TypeScript, with optional parentheses:

```
// Both valid:
if (x > 0) { process() }
if x > 0 { process() }

// If as expression
const sign = if x > 0 { 1 } else if x < 0 { -1 } else { 0 }

// Ternary (same as TypeScript)
const sign = x > 0 ? 1 : x < 0 ? -1 : 0
```

#### Match

Exhaustive pattern matching:

```
match value {
    0 => "zero"
    1 | 2 | 3 => "small"
    n if n < 0 => "negative"
    _ => "other"
}

match point {
    (0, 0) => "origin"
    (x, 0) => `x-axis at ${x}`
    (0, y) => `y-axis at ${y}`
    (x, y) => `at (${x}, ${y})`
}
```

#### Loops

```
for item in items { process(item) }
for i in 0..10 { print(i) }
while condition { /* ... */ }
loop { if done { break } }

// Traditional forms work too
for (let i = 0; i < 10; i++) { /* ... */ }
while (condition) { /* ... */ }
```

Labeled breaks:

```
outer: for i in 0..10 {
    for j in 0..10 {
        if condition { break outer }
    }
}
```

### Error Handling

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
const result = riskyOperation()?   // propagate error
const value = maybeNull!           // force unwrap (panic if null)
const value = maybeNull ?? default // coalesce (same as TypeScript)
```

### Other Expressions

```
defer file.close()                 // runs at scope exit
const data = await fetchData(url)  // same as TypeScript
yield value                        // same as TypeScript
return value
throw new Error("failed")
```

## Operators

Destack operators match TypeScript with additional, opt-in precision.
Destack adds wrapping/saturating variants for precise integer arithmetic:

| Op | Description | Wrapping | Saturating |
|----|-------------|----------|------------|
| `+` | Add | `+%` | `+\|` |
| `-` | Subtract | `-%` | `-\|` |
| `*` | Multiply | `*%` | `*\|` |
| `/` | Divide | | |
| `%` | Remainder | | |

Comparison, logical, bitwise, and assignment operators work like TypeScript.

## Patterns

Pattern matching extends TypeScript's destructuring:

```
_                    // wildcard
x                    // binding
42                   // literal
(a, b)               // tuple
{ x, y }             // struct/object
[first, ...rest]     // array
Ok(value)            // variant
x if x > 0           // guard
1..10                // range
```

## Modules

Module syntax matches TypeScript exactly:

```
import { foo, bar } from "module"
import * as mod from "module"
export const value = 42
export { foo as default }
```

## Literals

```
42                   // integer
3.14                 // float
0x1A, 0o17, 0b1010   // hex, octal, binary
1_000_000            // separators
"hello"              // string
'a'                  // character
`hello ${name}`      // template
r#"raw "string""#    // raw string
[1, 2, 3]            // array
(1, 2, 3)            // tuple
Point { x: 1, y: 2 } // struct
/\d+/g               // regex
```

### Tree Literals (TSX-like)

TSX works almost exactly like TypeScript/React, but covers general tree structures outside of UIs like entity trees, complex prompts or game objects.

```
<Entity id={1}>
    <Child name="foo" />
</Entity>
```

## Comments and Annotations

```
// line comment
/* block comment */
/// doc comment
/** block doc */

// #Performance: optimization note
// TODO: fix this
```

## Visibility

```
public field: int32
private field: int32
protected field: int32
#field: int32        // shorthand for private
```
