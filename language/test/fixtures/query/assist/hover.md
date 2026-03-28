# Hover

## Functions

### Hover over function name

Hovering over a function reference should display its signature.

```ds
function greet(name: string): string {
//         ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//            ^^^^^ use:greet
```

Hovering over `greet` at the call site should show "function greet".

```query hover use:greet
range=main.ds:5:13-5:18 signature=function greet(name: string): string documentation=<none>
```

### Hover over re-exported function

Hovering over a re-exported function should resolve to the original symbol.

```ds:lib.ds
export function announce(name: string): string {
    return "Hello, " + name;
}
```

```ds:barrel.ds
export { announce } from "./lib.ds";
```

```ds:main.ds
import { announce } from "./barrel.ds";

const message = announce("World");
//                ^^^^^^^^ use:announce
```

Hovering over `announce` should show "function announce".

```query hover use:announce
range=main.ds:3:17-3:25 signature=export function announce(name: string): string documentation=<none>
```

## Structs

### Hover over struct name

Hovering over a struct type reference should display its kind.

```ds
struct Point {
//       ^^^^^ def:Point
    x: float32
    y: float32
}

const p: Point = Point { x: 1, y: 2 };
//         ^^^^^ use:Point
```

Hovering over `Point` in the type annotation should show "struct Point".

```query hover use:Point
range=main.ds:6:10-6:15 signature=struct Point documentation=<none>
```

## Type Aliases

### Hover over type alias usage

Hovering over a type alias reference should display its kind.

```ds
type UserId = string;

function main() {
    const id: UserId = "abc";
//              ^^^^^^ use:UserId
}
```

Hovering over `UserId` should show "type UserId".

```query hover use:UserId
type UserId
```

## Classes

### Hover over class name

Hovering over a class type reference should display its kind.

```ds
class Animal {
//      ^^^^^^ def:Animal
    name: string
//    ^^^^ def:Animal.name
}

const a: Animal = new Animal();
//         ^^^^^^ use:Animal
```

Hovering over `Animal` in the type annotation should show "class Animal".

```query hover use:Animal
range=main.ds:5:10-5:16 signature=class Animal documentation=<none>
```

## Class Fields

### Hover over class field definition

Hovering over a class field definition should show its type.

```ds
class Person {
    name: string
//    ^^^^ def:name
    age: int32
//    ^^^ def:age
}
```

Hovering over `name` should show field info with container type.

```query hover def:name
range=main.ds:2:5-2:17 signature=(property) Person.name documentation=<none>
```

Hovering over `age` should show field info with container type.

```query hover def:age
range=main.ds:3:5-3:15 signature=(property) Person.age documentation=<none>
```

## Struct Fields

### Hover over struct field definition

Hovering over a struct field should show its type.

```ds
struct Vector2 {
    x: float32
//    ^ def:x
    y: float32
//    ^ def:y
}
```

Hovering over `x` should show field info with container type.

```query hover def:x
(property) Vector2.x
```

### Hover over struct field access

Hovering over a struct field access should show the same field signature.

```ds
struct Point {
    x: int32
    y: int32
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
//                     ^ use:point_x
}
```

Hovering over `p.x` should show the field info for `Point.x`.

```query hover use:point_x
(property) Point.x
```

## Methods

### Hover over method definition

Hovering over a method should show its signature.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
//    ^^^ def:add
        return a + b;
    }
}
```

Hovering over `add` should show method info with full signature.

```query hover def:add
(method) Calculator.add(a: int32, b: int32): int32
```

### Hover over method call

Hovering over a method call should resolve to the method signature.

```ds
class Logger {
    log(message: string): void {
        print(message);
    }
}

function main() {
    const logger = new Logger();
    logger.log("hello");
//           ^^^ use:logger_log
}
```

Hovering over `logger.log` should show the method signature.

```query hover use:logger_log
(method) Logger.log(message: string): void
```

## Enums

### Hover over enum field

Hovering over an enum variant should show enum info.

```ds
enum Color {
    Red,
//    ^^^ def:Red
    Green,
//    ^^^^^ def:Green
    Blue,
}
```

Hovering over `Red` should show enum member info with container type.

```query hover def:Red
(enum member) Color.Red
```

## Parameters

### Hover over function parameter

Hovering over a function parameter should show its type.

```ds
function multiply(x: int32, y: int32): int32 {
//                  ^ def:x_param
//                            ^ def:y_param
    return x * y;
}
```

Hovering over `x` parameter should show parameter info with type.

```query hover def:x_param
(parameter) x: int32
```

## Declaration Modifiers

Declaration spans should include prefix modifiers like `export`, `abstract`, and `declare`.

### Hover over export keyword on function

Hovering over the `export` keyword of an exported function should show the function.

```ds
export function greetExport(name: string): string {
//^^^^^^ hover:export_fn
//                ^^^^^^^^^^^ def:greetExport
    return "Hello, " + name;
}
```

Hovering over `export` should show function info because it's part of the declaration span.

```query hover hover:export_fn
export function greetExport(name: string): string
```

### Hover over export keyword on struct

Hovering over the `export` keyword of an exported struct should show the struct.

```ds
export struct ExportedPoint {
//^^^^^^ hover:export_struct
//              ^^^^^^^^^^^^^ def:ExportedPoint
    x: float32
    y: float32
}
```

```query hover hover:export_struct
export struct ExportedPoint
```

### Hover over export keyword on class

Hovering over the `export` keyword of an exported class should show the class.

```ds
export class ExportedAnimal {
//^^^^^^ hover:export_class
//             ^^^^^^^^^^^^^^ def:ExportedAnimal
    name: string
}
```

```query hover hover:export_class
export class ExportedAnimal
```

### Hover over export keyword on enum

Hovering over the `export` keyword of an exported enum should show the enum.

```ds
export enum ExportedColor {
//^^^^^^ hover:export_enum
//            ^^^^^^^^^^^^^ def:ExportedColor
    Red,
    Green,
    Blue,
}
```

```query hover hover:export_enum
export enum ExportedColor
```

### Hover over export keyword on interface

Hovering over the `export` keyword of an exported interface should show the interface.

```ds
export interface ExportedShape {
//^^^^^^ hover:export_interface
//                 ^^^^^^^^^^^^^ def:ExportedShape
    area(): float64
}
```

```query hover hover:export_interface
export interface ExportedShape
```

### Hover over export keyword on type alias

Hovering over the `export` keyword of an exported type alias should show the type.

```ds
export type ExportedId = string | int32
//^^^^^^ hover:export_type
//            ^^^^^^^^^^ def:ExportedId
```

```query hover hover:export_type
export type ExportedId
```

### Hover over abstract keyword on class

Hovering over the `abstract` keyword should show the class.

```ds
abstract class AbstractBase {
//^^^^^^^^ hover:abstract_class
//               ^^^^^^^^^^^^ def:AbstractBase
    abstract doSomething(): void
}
```

```query hover hover:abstract_class
abstract class AbstractBase
```

### Hover over export abstract combination

Hovering over `export` on an abstract class should show the class.

```ds
export abstract class ExportedAbstract {
//^^^^^^ hover:export_abstract
//                      ^^^^^^^^^^^^^^^^ def:ExportedAbstract
    abstract process(): void
}
```

```query hover hover:export_abstract
export abstract class ExportedAbstract
```

### Hover over declare keyword on function

Hovering over `declare` on an ambient declaration should show the declaration.

```ds
declare function declaredFn(x: int32): int32
//^^^^^^^ hover:declare_fn
//                 ^^^^^^^^^^ def:declaredFn
```

```query hover hover:declare_fn
declare function declaredFn(x: int32): int32
```

### Hover over export declare combination

Hovering over `export` on an ambient declaration should show the declaration.

```ds
export declare function exportDeclaredFn(x: int32): int32
//^^^^^^ hover:export_declare_fn
//                        ^^^^^^^^^^^^^^^^ def:exportDeclaredFn
```

```query hover hover:export_declare_fn
export declare function exportDeclaredFn(x: int32): int32
```

### Hover over declare keyword on class

Hovering over `declare` on an ambient class should show the class.

```ds
declare class DeclaredClass {
//^^^^^^^ hover:declare_class
//              ^^^^^^^^^^^^^ def:DeclaredClass
    constructor(name: string)
}
```

```query hover hover:declare_class
declare class DeclaredClass
```

### Hover over export keyword on namespace

Hovering over `export` on a namespace should show the namespace.

```ds
export namespace ExportedNS {
//^^^^^^ hover:export_ns
//                 ^^^^^^^^^^ def:ExportedNS
    export function inner(): void {}
}
```

```query hover hover:export_ns
export namespace ExportedNS
```

## Local Variables

### Hover over local variable

Hovering over a local variable should show its type.

```ds
struct Point {
    x: int32
    y: int32
}

function main() {
    const p: Point = Point { x: 1, y: 2 };
//          ^ hover:local_p
}
```

Hovering over `p` should show the variable with its type.

```query hover hover:local_p
let p: Point
```

### Hover over inferred local variable

Hovering over an inferred local variable should show the inferred type.

```ds
function main() {
    const count = 1;
//          ^^^^^ hover:local_count
}
```

```query hover hover:local_count
let count: 1
```

## Documentation Comments

### Hover on documentation should not return symbol

Hovering over a documentation comment should NOT return the symbol it documents.

```ds
class Person {
    /// The person's name.
//        ^^^^^^^^^^^^^^^^^ range:doc_span
    name: string
//    ^^^^ def:name_field
}
```

Hovering over the doc comment text should return nothing (no symbol).

```query hover range:doc_span
<none>
```

### Hover includes documentation text

Hovering over a documented symbol should include the doc comment text.

```ds
/// Says hello.
function greet(name: string): string {
    return name;
}

greet("World");
//^^^^^ use:greet
```

```query hover use:greet
range=main.ds:6:1-6:6 signature=function greet(name: string): string documentation=Says hello.
```

### Hover includes block documentation text

Block doc comments should also appear in hover documentation.

```ds
/**
 * Sends a greeting.
 */
function greetBlock(name: string): string {
    return name;
}

greetBlock("World");
//^^^^^^^^^^ use:greet_block
```

```query hover use:greet_block
range=main.ds:8:1-8:11 signature=function greetBlock(name: string): string documentation=Sends a greeting.
```

## Damaged Syntax

### Keep hover working after malformed function declarations

Hover should still resolve valid later declarations after one malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query hover use:stableLater
range=main.ds:5:1-5:12 signature=export function stableLater(): void documentation=<none>
```

### Keep hover working after malformed call statements

Hover should still resolve valid later declarations after one malformed call statement.

```ds
broken(,

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query hover use:stableLater
range=main.ds:5:1-5:12 signature=export function stableLater(): void documentation=<none>
```

### Keep hover working after bare new recovery statements

Hover should still resolve valid later declarations after one bare `new` recovery statement.

```ds
new

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query hover use:stableLater
range=main.ds:5:1-5:12 signature=export function stableLater(): void documentation=<none>
```

### Keep hover working after throw recovery statements

Hover should still resolve valid later declarations after one recovered `throw` statement.

```ds
throw

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query hover use:stableLater
range=main.ds:5:1-5:12 signature=export function stableLater(): void documentation=<none>
```

### Keep hover working after yield star recovery statements

Hover should still resolve valid later declarations after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query hover use:stableLater
range=main.ds:8:1-8:12 signature=export function stableLater(): void documentation=<none>
```

### Return no hover for malformed unresolved member access

Hover should return no symbol information when the cursor is on malformed unresolved syntax.

```ds
function main(): void {
    missingValue.
//    ^^^^^^^^^^^^ broken
}
```

```query hover broken
<none>
```

## Imports

### Hover over default imported function

Hovering over a default imported function should resolve to the original default export.

```ds:lib.ds
export default function greetDefault(name: string): string {
    return name;
}
```

```ds:main.ds
import greetDefault from "./lib.ds";

const message = greetDefault("hi");
//                ^^^^^^^^^^^^ use:default_greet
```

```query hover use:default_greet
range=main.ds:3:17-3:29 signature=export function greetDefault(name: string): string documentation=<none>
```

### Hover over namespace imported function

Hovering over a namespace imported function should resolve to the original exported symbol.

```ds:lib.ds
export function paint(color: string): string {
    return color;
}
```

```ds:main.ds
import * as palette from "./lib.ds";

const message = palette.paint("blue");
//                        ^^^^^ use:palette_paint
```

```query hover use:palette_paint
range=main.ds:3:25-3:30 signature=export function paint(color: string): string documentation=<none>
```

### Hover through default re-export chains

Hovering over a default import re-exported as a named symbol should still resolve to the original export.

```ds:lib.ds
export default function formatCount(value: int32): int32 {
    return value;
}
```

```ds:barrel.ds
export { default as formatCount } from "./lib.ds";
```

```ds:main.ds
import { formatCount } from "./barrel.ds";

const value = formatCount(1);
//              ^^^^^^^^^^^ use:format_count
```

```query hover use:format_count
range=main.ds:3:15-3:26 signature=export function formatCount(value: int32): int32 documentation=<none>
```
