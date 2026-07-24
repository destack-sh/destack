# Hover

## Functions

### Hover over a function reference

Hover shows the declaration signature at the selected reference.

```ds main.ds
function greet(name: string): string {
         ^^^^^ definition
    return name;
}

const message = greet("Destack");
                ^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="function greet(name: string): string" range=main.ds#reference
```

### Return the same declaration signature at its definition

Hovering the function name itself reports the authored declaration.

```ds main.ds
function greet(name: string): string {
         ^^^^^ definition
    return name;
}
```

```query hover main.ds#definition
@hover.result signature="function greet(name: string): string" range=main.ds#definition
```

### Return the selected overload

A call reports the exact overload selected by checking.

```ds main.ds
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const value = parse("one");
              ^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="function parse(value: string): string" range=main.ds#reference
```

### Include a selected generic instantiation

The authored generic signature and selected callable type remain distinct.

```ds main.ds
function identity<Value>(value: Value): Value {
         ^^^^^^^^ definition
    return value;
}

declare const name: string;
const result = identity(name);
               ^^^^^^^^ reference
```

```query hover main.ds#definition
@hover.result signature="function identity<Value>(value: Value): Value" range=main.ds#definition
```

```query hover main.ds#reference
@hover.result signature="function identity<Value>(value: Value): Value" type="identity<string>(value: string): string" range=main.ds#reference
```

## Documentation

### Include declaration documentation

Documentation comes from the referenced declaration.

```ds main.ds
/// Return the supplied name.
function identity(name: string): string {
         ^^^^^^^^ definition
    return name;
}

const name = identity("Destack");
             ^^^^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="function identity(name: string): string" documentation="Return the supplied name." range=main.ds#reference
```

### Return no symbol hover inside documentation

Documentation belongs to its declaration but is not itself a symbol occurrence.

```ds main.ds
/// Return the supplied name.
    ^^^^^^ documentation
function identity(name: string): string {
    return name;
}
```

```query hover main.ds#documentation
@hover.none
```

## Types

### Hover over a type alias

A type reference reports its declaration kind and name.

```ds main.ds
type UserId = int32;

declare const user: UserId;
                    ^^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="type UserId = int32" range=main.ds#reference
```

### Hover over a class

A class reference reports its declaration shape.

```ds main.ds
class Service {}

declare const service: Service;
                       ^^^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="class Service" range=main.ds#reference
```

### Hover over an interface

An interface reference reports its declaration shape.

```ds main.ds
interface Drawable {
    draw(): void;
}

declare const drawable: Drawable;
                        ^^^^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="interface Drawable" range=main.ds#reference
```

### Hover over a newtype

A newtype reference reports its nominal declaration.

```ds main.ds
newtype UserId = int64;

declare const user: UserId;
                    ^^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="newtype UserId = int64" range=main.ds#reference
```

### Hover over every remaining nominal declaration kind

Structs, enums, and nominal interfaces retain their distinct declaration signatures.

```ds main.ds
struct Packet {}
       ^^^^^^ packet

enum Status { Ready }
     ^^^^^^ status

newtype interface Display {}
                  ^^^^^^^ display
```

```query hover main.ds#packet
@hover.result signature="struct Packet" range=main.ds#packet
```

```query hover main.ds#status
@hover.result signature="enum Status" range=main.ds#status
```

```query hover main.ds#display
@hover.result signature="newtype interface Display" range=main.ds#display
```

### Include generic parameters in type declarations

A generic type reference reports the declaration's complete generic header.

```ds main.ds
class Box<Value> {}

declare const box: Box<int32>;
                   ^^^ reference
```

```query hover main.ds#reference
@hover.result signature="class Box<Value>" range=main.ds#reference
```

## Members

### Hover over a field access

A field access reports its owning type and field type.

```ds main.ds
struct Point {
    x: int32;
    ^ definition
}

function read(point: Point): int32 {
    return point.x;
                 ^ reference
}
```

```query hover main.ds#reference
@hover.result signature="(property) Point.x: int32" range=main.ds#reference
```

```query hover main.ds#definition
@hover.result signature="(property) Point.x: int32" range=main.ds#definition
```

### Hover over a method

A selected method reports its container-qualified signature.

```ds main.ds
class Service {
    run(value: int32): void {}
    ^^^ definition
}

function start(service: Service): void {
    service.run(1);
            ^^^ reference
}
```

```query hover main.ds#reference
@hover.result signature="(method) Service.run(value: int32): void" range=main.ds#reference
```

```query hover main.ds#definition
@hover.result signature="(method) Service.run(value: int32): void" range=main.ds#definition
```

### Hover over an extension method

An extension call reports the selected extension member and target type.

```ds main.ds
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
        return left + right;
    }
}

function total(calculator: Calculator): int32 {
    return calculator.add(1, 2);
                      ^^^ reference
}
```

```query hover main.ds#reference
@hover.result signature="(method) Calculator.add(left: int32, right: int32): int32" range=main.ds#reference
```

### Hover over an associated constant

An associated constant access reports its container and checked type.

```ds main.ds
struct Buffer {
    comptime const Width: uint = 8;
}

const width = Buffer.Width;
                     ^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="(comptime const) Buffer.Width: uint" range=main.ds#reference
```

### Include method documentation

Method hover includes documentation from the selected member.

```ds main.ds
class Service {
    /// Start one task.
    run(value: int32): void {}
}

function start(service: Service): void {
    service.run(1);
            ^^^ reference
}
```

```query hover main.ds#reference
@hover.result signature="(method) Service.run(value: int32): void" documentation="Start one task." range=main.ds#reference
```

### Hover over an enum member

An enum member reports its container-qualified value.

```ds main.ds
enum Color {
    Red,
}

const color = Color.Red;
                    ^^^ reference
```

```query hover main.ds#reference
@hover.result signature="(enum member) Color.Red: Color" range=main.ds#reference
```

### Hover over a tagged variant constructor

A tagged case reports the exact generated construction signature selected by checking.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;

const status = Status.Ok({ value: "ready" });
                      ^^ reference
```

```query hover main.ds#reference
@hover.result signature="(constructor) Status.Ok({ value: string }): Status" range=main.ds#reference
```

## Parameters

### Hover over a function parameter

A parameter reference reports its parameter type.

```ds main.ds
function identity(value: string): string {
    return value;
           ^^^^^ reference
}
```

```query hover main.ds#reference
@hover.result signature="(parameter) value: string" range=main.ds#reference
```

## Locals

### Hover over an inferred binding

A local binding reports its inferred type.

```ds main.ds
function read(): int32 {
    const count = 1;
          ^^^^^ reference
    return count;
}
```

```query hover main.ds#reference
@hover.result signature="const count: 1" range=main.ds#reference
```

## Imports

### Hover over an imported function

Imported references use the declaration signature from the defining module.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { greet } from "./library.ds";

greet();
^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="export function greet(): void" range=main.ds#reference
```

### Hover through a re-export

A re-exported function retains its declaration signature and documentation.

```ds library.ds
/// Scale one value.
export default function scale(value: int32, factor: int32): int32 {
    return value * factor;
}
```

```ds barrel.ds
export { default as scale } from "./library.ds";
```

```ds main.ds
import { scale } from "./barrel.ds";

const result = scale(2, 3);
               ^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="export default function scale(value: int32, factor: int32): int32" documentation="Scale one value." range=main.ds#reference
```

### Hover over a default import

A default import retains the declaration signature from its defining module.

```ds library.ds
export default function greet(name: string): string {
    return name;
}
```

```ds main.ds
import welcome from "./library.ds";

const message = welcome("Destack");
                ^^^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="export default function greet(name: string): string" range=main.ds#reference
```

### Hover over a namespace member

A namespace member retains the declaration signature from its defining module.

```ds library.ds
export function greet(name: string): string {
    return name;
}
```

```ds main.ds
import * as library from "./library.ds";

const message = library.greet("Destack");
                        ^^^^^ reference
```

```query hover main.ds#reference
@hover.result signature="export function greet(name: string): string" range=main.ds#reference
```

## Empty Results

### Return no hover for a literal

An ordinary literal has no symbol hover.

```ds main.ds
const value = 42;
              ^^ literal
```

```query hover main.ds#literal
@hover.none
```
