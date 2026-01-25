# Completion

## Basic Keywords

### Complete with keywords at statement position

At any statement position, completion should offer language keywords.

```ds
const foo = 1;
$0
```

```query completion $0
- function: keyword
- const: keyword
- struct: keyword
```

### Complete in empty file

Even in an empty file, completion should offer keywords.

```ds
$0
```

```query completion $0
- function: keyword
- const: keyword
```

## Type Position

### Complete primitives after colon

In type annotation position (after `:`), should show primitive types but NOT keywords.

```ds
const x: $0
```

```query completion $0
- int32: type_parameter
- string: type_parameter
- bool: type_parameter
! function: keyword
! const: keyword
```

### Complete after extends

After `extends` keyword, should show primitive types but NOT keywords.

```ds
class Child extends $0
```

```query completion $0
- int32: type_parameter
- string: type_parameter
! function: keyword
! const: keyword
```

### Complete declared types in type position

User-defined types should appear in type position.

```ds
struct Point {
    x: int32,
    y: int32,
}

function test() {
    const p: $0 = Point { x: 0, y: 0 };
}
```

```query completion $0
- Point: struct
- int32: type_parameter
- string: type_parameter
```

### Complete in generic type parameter

In generic brackets, should show types.

```ds
struct Container<T> {
    value: T,
}

struct Point { x: int32, y: int32 }

function test() {
    const c: Container<$0> = Container { 
        value: Point { x: 0, y: 0 } 
    };
}
```

```query completion $0
- Point: struct
- int32: type_parameter
```

## Import Paths

### Suggest relative starters for empty import

When completing an empty import path, suggest relative path starters.

```ds
import { } from "$0"
```

```query completion $0
- ./: folder
- ../: folder
```

## Member Access

### Complete struct name reference

In value position, struct names should be available as completions.

```ds
struct Point {
    x: int32,
    y: int32,
}

function main() {
    $0
}
```

```query completion $0
- Point: struct
- main: function
```

### Complete after dot

When triggering completion immediately after a dot, show all members.

```ds
struct Point2 {
    x: int32,
    y: int32,
}

function main2() {
    const p: Point2 = Point2 { x: 1, y: 2 };
    p.$0
}
```

```query completion $0
- x: field
- y: field
```

### Complete partial member name

When typing a partial member name after a dot, show matching members.

```ds
struct Point {
    xa: int32,
    xb: int32,
    y: int32,
}

function main() {
    const p: Point = Point { xa: 1, xb: 2, y: 3 };
    p.x$0
}
```

```query completion $0
- xa: field
- xb: field
```

### Complete class methods

Class methods should appear after dot.

```ds
class Calculator {
    function add(a: int32, b: int32): int32 {
        return a + b;
    }

    function multiply(a: int32, b: int32): int32 {
        return a * b;
    }
}

function main() {
    const calc = new Calculator();
    calc.$0
}
```

```query completion $0
- add: method
- multiply: method
```

## Object Literal

### Complete fields in empty object literal

When inside an empty object literal, suggest all fields.

```ds
struct Point {
    x: int32,
    y: int32,
}

function main() {
    const p: Point = Point { $0 };
}
```

```query completion $0
- x: field
- y: field
```

### Complete remaining fields in object literal

When inside an object literal with a known type, suggest remaining fields.

```ds
struct Config {
    host: string,
    port: int32,
    timeout: int32,
}

function main() {
    const cfg: Config = Config { host: "localhost", $0 };
}
```

```query completion $0
- port: field
- timeout: field
! host: field
```

### Complete fields in nested object literal

Nested object literals should suggest fields of the nested type.

```ds
struct Address {
    street: string,
    city: string,
}

struct Person {
    name: string,
    address: Address,
}

function main() {
    const person: Person = Person {
        name: "John",
        address: Address { $0 },
    };
}
```

```query completion $0
- street: field
- city: field
```

## Scope and Variables

### Complete local variables

Local variables should be available in completions.

```ds
function test() {
    const name = "hello";
    const count = 42;
    $0
}
```

```query completion $0
- name: variable
- count: variable
- test: function
```

### Complete variables from outer scope

Variables from enclosing scopes should be visible.

```ds
function outer() {
    const outerVar = 1;

    function inner() {
        const innerVar = 2;
        $0
    }
}
```

```query completion $0
- innerVar: variable
- outerVar: variable
- inner: function
- outer: function
```

### Complete function parameters

Function parameters should be available inside the function body.

```ds
function greet(name: string, count: int32) {
    $0
}
```

```query completion $0
- name: variable
- count: variable
- greet: function
```

### Shadowed variables show inner binding

When a variable shadows another, the inner one should be preferred.

```ds
function test() {
    const x = "outer";
    {
        const x = 42;
        $0
    }
}
```

```query completion $0
- x: variable
```

## Fuzzy Matching

### Fuzzy match with prefix

Exact prefix matches should have highest priority.

```ds
function toString() {}
function toNumber() {}
function fromString() {}

function main() {
    to$0
}
```

```query completion $0
- toString: function
- toNumber: function
! fromString: function
```

### Fuzzy match case insensitive

Case-insensitive prefix should still match.

```ds
function ToString() {}
function ToNumber() {}

function main() {
    to$0
}
```

```query completion $0
- ToString: function
- ToNumber: function
```

### Fuzzy match camelCase boundaries

Typing initials should match camelCase symbols.

```ds
function getElementsByClassName() {}
function getElementById() {}
function querySelector() {}

function main() {
    geb$0
}
```

```query completion $0
- getElementsByClassName: function
- getElementById: function
! querySelector: function
```

### Fuzzy match substring

Characters appearing in order should match.

```ds
function completion() {}
function configuration() {}
function connection() {}

function main() {
    cmpl$0
}
```

```query completion $0
- completion: function
```

## Enums

### Complete enum variants in type position

Enum names should appear in type position.

```ds
enum Color {
    Red,
    Green,
    Blue,
}

function test() {
    const c: $0 = Color.Red;
}
```

```query completion $0
- Color: enum
```

### Complete enum members after dot

Enum members should appear after the enum name.

```ds
enum Status {
    Pending,
    Active,
    Completed,
}

function main() {
    const s = Status.$0
}
```

```query completion $0
- Pending: enum_member
- Active: enum_member
- Completed: enum_member
```

## Function Arguments

### Complete in function argument position

In function call arguments, show value completions.

```ds
function greet(name: string) {}

function main() {
    const userName = "Alice";
    greet($0)
}
```

```query completion $0
- userName: variable
- main: function
- greet: function
```

## Interface Members

### Complete interface method implementations

When implementing an interface, suggest required members.

```ds
interface Drawable {
    function draw(): void;
    function getArea(): float64;
}

class Circle implements Drawable {
    $0
}
```

```query completion $0
- function: keyword
```

## Newtypes

### Complete newtype name in type position

Newtype names should appear in type position like other nominal types.

```ds
newtype UserId = int64;
newtype OrderId = int64;

function test() {
    const id: $0 = UserId(1);
}
```

```query completion $0
- UserId: type_parameter
- OrderId: type_parameter
- int64: type_parameter
```

## Generics

### Complete generic struct fields

Generic struct instantiations should complete their fields.

```ds
struct Container<T> {
    value: T,
    count: int32,
}

function main() {
    const c: Container<string> = Container { value: "hello", count: 1 };
    c.$0
}
```

```query completion $0
- value: field
- count: field
```

## Extensions

### Complete extension methods

Extension methods should appear after dot on extended types.

```ds
struct Point {
    x: number,
    y: number,
}

extension for Point {
    length(): number { return 0 }
    normalized(): Point { return Point { x: 0, y: 0 } }
}

function main() {
    const p = Point { x: 1, y: 2 };
    p.$0
}
```

```query completion $0
- x: field
- y: field
- length: method
- normalized: method
```

### Complete extension methods with partial prefix

Extension methods should be filtered by prefix.

```ds
struct Vector {
    x: number,
    y: number,
}

extension for Vector {
    lengthSquared(): number { return 0 }
    length(): number { return 0 }
    normalize(): Vector { return Vector { x: 0, y: 0 } }
}

function main() {
    const v = Vector { x: 1, y: 2 };
    v.len$0
}
```

```query completion $0
- lengthSquared: method
- length: method
! normalize: method
```

### Complete interface implementation via extension

Interface methods from extension implementations should appear.

```ds
interface Describable {
    describe(): string;
}

struct Item {
    name: string,
}

extension for Item implements Describable {
    describe(): string { return "" }
}

function main() {
    const item = Item { name: "test" };
    item.$0
}
```

```query completion $0
- name: field
- describe: method
```

## Additional Newtype Tests

### Complete newtype methods via extension

Extension methods on newtypes should appear after dot.

```ds
newtype Email = string;

extension for Email {
    domain(): string { return "" }
    localPart(): string { return "" }
}

function main() {
    const email = Email("test@example.com");
    email.$0
}
```

```query completion $0
- domain: method
- localPart: method
```
