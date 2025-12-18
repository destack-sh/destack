# Hover

## Functions

### Hover over function name

Hovering over a function reference should display its signature.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//          ^^^^^ use:greet
```

Hovering over `greet` at the call site should show "function greet".

```query hover use:greet
function greet
```

## Structs

### Hover over struct name

Hovering over a struct type reference should display its kind.

```ds
struct Point {
//     ^^^^^ def:Point
    x: float32,
    y: float32,
}

const p: Point = Point { x: 1, y: 2 };
//       ^^^^^ use:Point
```

Hovering over `Point` in the type annotation should show "struct Point".

```query hover use:Point
struct Point
```

## Classes

### Hover over class name

Hovering over a class type reference should display its kind.

```ds
class Animal {
//    ^^^^^^ def:Animal
    name: string
//  ^^^^ def:Animal.name
}

const a: Animal = new Animal();
//       ^^^^^^ use:Animal
```

Hovering over `Animal` in the type annotation should show "class Animal".

```query hover use:Animal
class Animal
```

## Class Fields

### Hover over class field definition

Hovering over a class field definition should show its type.

```ds
class Person {
    name: string
//  ^^^^ def:name
    age: int32
//  ^^^ def:age
}
```

Hovering over `name` should show field info.

```query hover def:name
field name
```

Hovering over `age` should show field info.

```query hover def:age
field age
```

## Struct Fields

### Hover over struct field definition

Hovering over a struct field should show its type.

```ds
struct Vector2 {
    x: float32,
//  ^ def:x
    y: float32,
//  ^ def:y
}
```

Hovering over `x` should show field info.

```query hover def:x
field x
```

## Methods

### Hover over method definition

Hovering over a method should show its signature.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
//  ^^^ def:add
        return a + b;
    }
}
```

Hovering over `add` should show method info.

```query hover def:add
method add
```

## Enums

### Hover over enum field

Hovering over an enum variant should show enum info.

```ds
enum Color {
    Red,
//  ^^^ def:Red
    Green,
//  ^^^^^ def:Green
    Blue,
}
```

Hovering over `Red` should show enum field info.

```query hover def:Red
enum field Red
```

## Parameters

### Hover over function parameter

Hovering over a function parameter should show its type.

```ds
function multiply(x: int32, y: int32): int32 {
//                ^ def:x_param
//                          ^ def:y_param
    return x * y;
}
```

Hovering over `x` parameter should show parameter info.

```query hover def:x_param
parameter x
```

## Declaration Modifiers

Declaration spans should include prefix modifiers like `export`, `abstract`, and `declare`.

### Hover over export keyword on function

Hovering over the `export` keyword of an exported function should show the function.

```ds
export function greetExport(name: string): string {
//^^^^ hover:export_fn
//              ^^^^^^^^^^^ def:greetExport
    return "Hello, " + name;
}
```

Hovering over `export` should show function info because it's part of the declaration span.

```query hover hover:export_fn
function greetExport
```

### Hover over export keyword on struct

Hovering over the `export` keyword of an exported struct should show the struct.

```ds
export struct ExportedPoint {
//^^^^ hover:export_struct
//            ^^^^^^^^^^^^^ def:ExportedPoint
    x: float32,
    y: float32,
}
```

```query hover hover:export_struct
struct ExportedPoint
```

### Hover over export keyword on class

Hovering over the `export` keyword of an exported class should show the class.

```ds
export class ExportedAnimal {
//^^^^ hover:export_class
//           ^^^^^^^^^^^^^^ def:ExportedAnimal
    name: string
}
```

```query hover hover:export_class
class ExportedAnimal
```

### Hover over export keyword on enum

Hovering over the `export` keyword of an exported enum should show the enum.

```ds
export enum ExportedColor {
//^^^^ hover:export_enum
//          ^^^^^^^^^^^^^ def:ExportedColor
    Red,
    Green,
    Blue,
}
```

```query hover hover:export_enum
enum ExportedColor
```

### Hover over export keyword on interface

Hovering over the `export` keyword of an exported interface should show the interface.

```ds
export interface ExportedShape {
//^^^^ hover:export_interface
//               ^^^^^^^^^^^^^ def:ExportedShape
    area(): float64
}
```

```query hover hover:export_interface
interface ExportedShape
```

### Hover over export keyword on type alias

Hovering over the `export` keyword of an exported type alias should show the type.

```ds
export type ExportedId = string | int32
//^^^^ hover:export_type
//          ^^^^^^^^^^ def:ExportedId
```

```query hover hover:export_type
type ExportedId
```

### Hover over abstract keyword on class

Hovering over the `abstract` keyword should show the class.

```ds
abstract class AbstractBase {
//^^^^^^ hover:abstract_class
//             ^^^^^^^^^^^^ def:AbstractBase
    abstract doSomething(): void
}
```

```query hover hover:abstract_class
class AbstractBase
```

### Hover over export abstract combination

Hovering over `export` on an abstract class should show the class.

```ds
export abstract class ExportedAbstract {
//^^^^ hover:export_abstract
//                    ^^^^^^^^^^^^^^^^ def:ExportedAbstract
    abstract process(): void
}
```

```query hover hover:export_abstract
class ExportedAbstract
```

### Hover over declare keyword on function

Hovering over `declare` on an ambient declaration should show the declaration.

```ds
declare function declaredFn(x: int32): int32
//^^^^^ hover:declare_fn
//               ^^^^^^^^^^ def:declaredFn
```

```query hover hover:declare_fn
function declaredFn
```

### Hover over export declare combination

Hovering over `export` on an ambient declaration should show the declaration.

```ds
export declare function exportDeclaredFn(x: int32): int32
//^^^^ hover:export_declare_fn
//                      ^^^^^^^^^^^^^^^^ def:exportDeclaredFn
```

```query hover hover:export_declare_fn
function exportDeclaredFn
```

### Hover over declare keyword on class

Hovering over `declare` on an ambient class should show the class.

```ds
declare class DeclaredClass {
//^^^^^ hover:declare_class
//            ^^^^^^^^^^^^^ def:DeclaredClass
    constructor(name: string)
}
```

```query hover hover:declare_class
class DeclaredClass
```

### Hover over export keyword on namespace

Hovering over `export` on a namespace should show the namespace.

```ds
export namespace ExportedNS {
//^^^^ hover:export_ns
//               ^^^^^^^^^^ def:ExportedNS
    export function inner(): void {}
}
```

```query hover hover:export_ns
namespace ExportedNS
```

## Documentation Comments

### Hover on documentation should not return symbol

Hovering over a documentation comment should NOT return the symbol it documents.

```ds
class Person {
    /// The person's name.
//      ^^^^^^^^^^^^^^^^^ hover:doc_span
    name: string
//  ^^^^ def:name_field
}
```

Hovering over the doc comment text should return nothing (no symbol).

```query hover hover:doc_span
<none>
```
