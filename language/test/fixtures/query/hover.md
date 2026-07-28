# Hover

## Functions

### Hover over a function reference

Hover shows a declaration signature at its reference.

```ds main.ds
function greet(name: string): string {
         ^^^^^ definition
    return name;
}

const message = greet("Destack");
                ^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="function greet(name: string): string" location=main.ds:1:1-3:2 selection=main.ds#definition range=main.ds#reference
```

### Return the same declaration signature at its definition

Hovering a function name reports its declaration.

```ds main.ds
function greet(name: string): string {
         ^^^^^ definition
    return name;
}
```

```query hover main.ds#definition
@hover.item index=0 signature="function greet(name: string): string" location=main.ds:1:1-3:2 selection=main.ds#definition range=main.ds#definition
```

### Return the matching overload

A call reports the overload that accepts its arguments.

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
@hover.item index=0 signature="function parse(value: string): string" location=main.ds:5:1-7:2 selection=main.ds:5:10-5:15 range=main.ds#reference
```

### [ignored] Return an overload family

A value reference to an overload family reports every declaration.

```ds main.ds
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const parser = parse;
               ^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="function parse(value: int32): int32" location=main.ds:1:1-3:2 selection=main.ds:1:10-1:15 range=main.ds#reference
@hover.item index=1 signature="function parse(value: string): string" location=main.ds:5:1-7:2 selection=main.ds:5:10-5:15 range=main.ds#reference
```

### Include an applied generic instantiation

The generic declaration and applied callable type remain distinct.

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
@hover.item index=0 signature="function identity<Value>(value: Value): Value" location=main.ds:1:1-3:2 selection=main.ds#definition range=main.ds#definition
```

```query hover main.ds#reference
@hover.item index=0 signature="function identity<Value>(value: Value): Value" type="(value: string) => string" location=main.ds:1:1-3:2 selection=main.ds#definition range=main.ds#reference
```

## Documentation

### [ignored] Include declaration documentation

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
@hover.item index=0 signature="function identity(name: string): string" documentation="Return the supplied name." location=main.ds:2:1-4:2 selection=main.ds#definition range=main.ds#reference
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
@hover.item index=0 signature="type UserId = int32" type=UserId location=main.ds:1:1-1:20 selection=main.ds:1:6-1:12 range=main.ds#reference
```

### Hover over a class

A class reference reports its declaration shape.

```ds main.ds
class Service {}

declare const service: Service;
                       ^^^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="class Service" location=main.ds:1:1-1:17 selection=main.ds:1:7-1:14 range=main.ds#reference
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
@hover.item index=0 signature="interface Drawable" location=main.ds:1:1-3:2 selection=main.ds:1:11-1:19 range=main.ds#reference
```

### Hover over a newtype

A newtype reference reports its nominal declaration.

```ds main.ds
newtype UserId = int64;

declare const user: UserId;
                    ^^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="newtype UserId = int64" location=main.ds:1:1-1:23 selection=main.ds:1:9-1:15 range=main.ds#reference
```

### Hover over every remaining nominal declaration kind

Structs, enums, and nominal interfaces report distinct declaration signatures.

```ds main.ds
struct Packet {}
       ^^^^^^ packet

enum Status { Ready }
     ^^^^^^ status

newtype interface Display {}
                  ^^^^^^^ display
```

```query hover main.ds#packet
@hover.item index=0 signature="struct Packet" location=main.ds:1:1-1:17 selection=main.ds#packet range=main.ds#packet
```

```query hover main.ds#status
@hover.item index=0 signature="enum Status" location=main.ds:3:1-3:22 selection=main.ds#status range=main.ds#status
```

```query hover main.ds#display
@hover.item index=0 signature="newtype interface Display" location=main.ds:5:1-5:29 selection=main.ds#display range=main.ds#display
```

### Include generic parameters in type declarations

A generic type reference reports the declaration's complete generic header.

```ds main.ds
class Box<Value> {}

declare const box: Box<int32>;
                   ^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="class Box<Value>" type="Box<int32>" location=main.ds:1:1-1:20 selection=main.ds:1:7-1:10 range=main.ds#reference
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
@hover.item index=0 signature="(property) Point.x: int32" location=main.ds:2:5-2:13 selection=main.ds#definition range=main.ds#reference
```

```query hover main.ds#definition
@hover.item index=0 signature="(property) Point.x: int32" location=main.ds:2:5-2:13 selection=main.ds#definition range=main.ds#definition
```

### Hover over a method

A method reports its container-qualified signature.

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
@hover.item index=0 signature="(method) Service.run(value: int32): void" location=main.ds:2:5-2:31 selection=main.ds#definition range=main.ds#reference
```

```query hover main.ds#definition
@hover.item index=0 signature="(method) Service.run(value: int32): void" location=main.ds:2:5-2:31 selection=main.ds#definition range=main.ds#definition
```

### Hover over an extension method

An extension call reports its method and target type.

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
@hover.item index=0 signature="(method) Calculator.add(left: int32, right: int32): int32" location=main.ds:4:5-6:6 selection=main.ds:4:5-4:8 range=main.ds#reference
```

### Hover over an associated constant

An associated constant access reports its container and type.

```ds main.ds
struct Buffer {
    comptime const Width: uint = 8;
}

const width = Buffer.Width;
                     ^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="(comptime const) Buffer.Width: uint64" location=main.ds:2:5-2:35 selection=main.ds:2:20-2:25 range=main.ds#reference
```

### [ignored] Include method documentation

Method hover includes documentation from the member declaration.

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
@hover.item index=0 signature="(method) Service.run(value: int32): void" documentation="Start one task." location=main.ds:3:5-3:31 selection=main.ds:3:5-3:8 range=main.ds#reference
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
@hover.item index=0 signature="(enum member) Color.Red: Color.Red" location=main.ds:2:5-2:8 range=main.ds#reference
```

### [ignored] Hover over a tagged variant constructor

A tagged case reports its constructor signature.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;

const status = Status.Ok({ value: "ready" });
                      ^^ reference
```

```query hover main.ds#reference
@hover.item index=0 signature="(constructor) Status.Ok({ value: string }): Status" location=main.ds:2:18-2:28 selection=main.ds:2:18-2:20 range=main.ds#reference
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
@hover.item index=0 signature="(parameter) value: string" location=main.ds:1:19-1:32 selection=main.ds:1:19-1:24 range=main.ds#reference
```

## Locals

### Hover over an inferred binding

A local binding reports its inferred type.

```ds main.ds
function read(): int32 {
    const count = 1;
          ^^^^^ definition
    return count;
           ^^^^^ reference
}
```

```query hover main.ds#reference
@hover.item index=0 signature="const count: 1" location=main.ds#definition range=main.ds#reference
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
@hover.item index=0 signature="export function greet(): void" location=library.ds:1:1-1:33 selection=library.ds:1:17-1:22 range=main.ds#reference
```

### Hover through a re-export

A re-export resolves to the original declaration.

```ds library.ds
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
@hover.item index=0 signature="export default function scale(value: int32, factor: int32): int32" location=library.ds:1:1-3:2 selection=library.ds:1:25-1:30 range=main.ds#reference
```

### [ignored] Include documentation through a re-export

A re-exported function reports its declaration signature and documentation.

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
@hover.item index=0 signature="export default function scale(value: int32, factor: int32): int32" documentation="Scale one value." location=library.ds:2:1-4:2 selection=library.ds:2:25-2:30 range=main.ds#reference
```

### Hover over a default import

A default import reports the declaration signature from its defining module.

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
@hover.item index=0 signature="export default function greet(name: string): string" location=library.ds:1:1-3:2 selection=library.ds:1:25-1:30 range=main.ds#reference
```

### Hover over a namespace member

A namespace member reports the declaration signature from its defining module.

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
@hover.item index=0 signature="export function greet(name: string): string" location=library.ds:1:1-3:2 selection=library.ds:1:17-1:22 range=main.ds#reference
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
