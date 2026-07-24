# Completion

## Local Symbols

### Complete a local binding prefix

The matching local symbol ranks first.

```ds main.ds
const alpha = 1;
const result = alpha;
               ^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=alpha kind=constant detail=1 sort=10 preselect=true matches=0,1,2,3,4
```

### Exclude a later local declaration

A declaration is not visible before its lexical declaration point.

```ds main.ds
function read(): void {
    futureScope
    ^^^^^^^^^^^ prefix

    const futureScopeValue = 1;
}
```

```query completion main.ds#prefix@end
@completion.none
```

### Complete a function parameter

Parameters remain visible throughout their function body.

```ds main.ds
function calculate(totalValue: int32): int32 {
    return totalV;
           ^^^^^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=totalValue kind=variable detail=int32 sort=10 preselect=true matches=0,1,2,3,4,5
```

### Complete an outer binding

Nested scopes retain visible bindings from their parents.

```ds main.ds
const outerValue = 1;

function read(): int32 {
    return outerV;
           ^^^^^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=outerValue kind=constant detail=1 sort=10 preselect=true matches=0,1,2,3,4,5
```

### Prefer the nearest shadowing declaration

One visible name produces one item using the innermost declaration.

```ds main.ds
const targetValue: string = "";

function read(): int32 {
    const targetValue: int32 = 1;
    return targetV;
           ^^^^^^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=targetValue kind=constant detail=int32 sort=10 preselect=true matches=0,1,2,3,4,5,6
```

### Complete a function call

A function completion includes its checked signature, documentation, and call snippet.

```ds main.ds
/// Format one name.
function formatName(name: string, width: int32): string {
    return name;
}

const result = formatN;
               ^^^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=formatName kind=function detail="(name: string, width: int32) => string" documentation="Format one name." insert="formatName(${1:name}, ${2:width})$0" sort=10 snippet=true preselect=true matches=0,1,2,3,4,5,6
```

### Match a camel-case prefix

Lexical matching retains the exact character positions used for ranking and highlighting.

```ds main.ds
const currentValue = 1;
const result = cv;
               ^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=currentValue kind=constant detail=1 sort=10 preselect=true matches=0,7
```

## Members

### Complete struct fields

Member completion lists the receiver's fields.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

function read(point: Point): int32 {
    return point.x;
                 ^ member
}
```

```query completion main.ds#member@end trigger=.
@completion.item label=x kind=field detail=int32 sort=10 preselect=true matches=0
```

### Complete an instance method

Method completion includes its checked callable type and insertion snippet.

```ds main.ds
class Buffer {
    /// Read one byte.
    read(index: uint): uint8 {
        return 0;
    }
}

function read(buffer: Buffer): uint8 {
    return buffer.re;
                  ^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=read kind=method detail="(index: uint) => uint8" documentation="Read one byte." insert="read(${1:index})$0" sort=10 snippet=true preselect=true matches=0,1
```

### Separate static and instance members

Type receivers expose static members and value receivers expose instance members.

```ds main.ds
class Buffer {
    static create(size: uint): Buffer {
        return new Buffer();
    }

    clear(): void {}
}

const buffer = Buffer.cr;
                      ^^ static_prefix

declare const value: Buffer;
value.cr;
      ^^ instance_prefix
```

```query completion main.ds#static_prefix@end trigger=.
@completion.item label=create kind=method detail="(size: uint) => Buffer" insert="create(${1:size})$0" sort=10 snippet=true preselect=true matches=0,1
```

```query completion main.ds#instance_prefix@end trigger=.
@completion.none
```

### Complete an extension method

Member completion includes extension methods for the receiver type.

```ds main.ds
struct Calculator {}

extension of Calculator {
    sum(left: int32, right: int32): int32 {
        return left + right;
    }
}

function calculate(value: Calculator): int32 {
    return value.su;
                 ^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=sum kind=method detail="(left: int32, right: int32) => int32" insert="sum(${1:left}, ${2:right})$0" sort=10 snippet=true preselect=true matches=0,1
```

### Complete an optional-chain member

Optional chaining completes the same members as direct access.

```ds main.ds
struct Point {
    x: int32;
}

function read(point: Point | undefined): int32 | undefined {
    return point?.x;
                  ^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=x kind=field detail=int32 sort=10 preselect=true matches=0
```

### Complete a generic field

A generic field uses the receiver's applied type arguments.

```ds main.ds
struct Box<Value> {
    value: Value;
}

function read(box: Box<string>): string {
    return box.val;
               ^^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=value kind=field detail=string sort=10 preselect=true matches=0,1,2
```

### Complete a common union member once

Union member completion includes only members available on every possible receiver.

```ds main.ds
struct Circle {
    label: string;
    radius: float64;
}

struct Square {
    label: string;
    width: float64;
}

function label(shape: Circle | Square): string {
    return shape.la;
                 ^^ common_prefix
}

function radius(shape: Circle | Square): float64 {
    return shape.ra;
                 ^^ partial_prefix
}
```

```query completion main.ds#common_prefix@end trigger=.
@completion.item label=label kind=field detail=string sort=10 preselect=true matches=0,1
```

```query completion main.ds#partial_prefix@end trigger=.
@completion.none
```

### Complete an associated constant

A nominal type receiver exposes its associated values.

```ds main.ds
struct Buffer {
    comptime const Width: uint = 8;
}

const width = Buffer.Wi;
                     ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Width kind=constant detail=uint sort=10 preselect=true matches=0,1
```

### Complete a namespace member

A namespace receiver exposes the exports of its resolved module.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import * as library from "./library.ds";

library.gr;
        ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=greet kind=function detail="() => void" insert="greet()$0" sort=10 snippet=true preselect=true matches=0,1
```

## Implementations

### Complete a required method

A class body can insert a missing interface method as one complete declaration.

```ds main.ds
interface Service {
    run(value: int32): void;
}

class Application implements Service {
    ru
    ^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=run kind=method detail="Implement Service.run" insert="run(value: int32): void {\n    $0\n}" sort=10 snippet=true preselect=true matches=0,1
```

### Omit methods that are already implemented

Implementation completion lists only missing required members.

```ds main.ds
interface Service {
    run(value: int32): void;
    stop(): void;
}

class Application implements Service {
    run(value: int32): void {}

    ru
    ^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.none
```

## Types

### Complete a nominal type

Type positions include visible type declarations.

```ds main.ds
struct Point {
    x: int32;
}

declare const point: Poi;
                     ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=Point kind=struct sort=10 preselect=true matches=0,1,2
```

### Exclude value-only declarations from a type position

Type completion follows the language namespaces rather than returning lexical name matches.

```ds main.ds
const PacketValue = 1;

declare const packet: PacketV;
                      ^^^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.none
```

### Complete a generic type parameter

A generic parameter remains visible throughout its declaration.

```ds main.ds
function identity<Value>(value: Value): Val {
                                       ^^^ prefix
    return value;
}
```

```query completion main.ds#prefix@end
@completion.item label=Value kind=type_parameter sort=10 preselect=true matches=0,1,2
```

### Complete an imported type through a re-export

Type completion retains the imported declaration kind through module aliases.

```ds model.ds
export struct Packet {}
```

```ds library.ds
export { Packet } from "./model.ds";
```

```ds main.ds
import { Packet } from "./library.ds";

declare const packet: Pack;
                      ^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=Packet kind=struct sort=10 preselect=true matches=0,1,2,3
```

## Enum Members

### Complete an enum member

Member completion uses the enum receiver type.

```ds main.ds
enum Color {
    Red,
    Blue,
}

const color = Color.R;
                    ^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Red kind=enum_member detail=Color sort=10 preselect=true matches=0
```

### Complete a tagged variant

A tagged case completion carries its payload constructor and result type.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string> | Error<int32>;

const status = Status.O;
                      ^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Ok kind=constructor detail="({ value: string }) => Status" insert="Ok({ value: ${1} })$0" sort=10 snippet=true preselect=true matches=0
```

## Constructors

### Complete a constructable class

New expressions include constructable nominal values.

```ds main.ds
class Widget {
    constructor(name: string) {}
}

const widget = new Widget;
                   ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=Widget kind=class detail="new (name: string) => Widget" insert="Widget(${1:name})$0" sort=10 snippet=true preselect=true matches=0,1,2
```

### Complete a struct expression

A struct value completion can insert every required field.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

const point = Poi;
              ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=Point kind=struct detail=Point insert="Point { x: ${1}, y: ${2} }$0" sort=10 snippet=true preselect=true matches=0,1,2
```

### Complete a newtype constructor

A newtype value completion exposes its backing value parameter.

```ds main.ds
newtype UserId = string;

const userId = UserI;
               ^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=UserId kind=constructor detail="(value: string) => UserId" insert="UserId(${1:value})$0" sort=10 snippet=true preselect=true matches=0,1,2,3,4
```

## Object Literals

### Complete a missing field

Object completion omits fields already present in the literal.

```ds main.ds
struct Rectangle {
    width: int32;
    height: int32;
}

const rectangle: Rectangle = {
    width: 10,
    hei
    ^^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.item label=height kind=field detail=int32 insert="height: ${1}" sort=10 preselect=true snippet=true matches=0,1,2
```

### Complete a nested field

Nested object literals use the expected type at their own position.

```ds main.ds
struct Address {
    street: string;
}

struct User {
    address: Address;
}

const user: User = {
    address: {
        str
        ^^^ prefix
    },
};
```

```query completion main.ds#prefix@end
@completion.item label=street kind=field detail=string insert="street: ${1}" sort=10 preselect=true snippet=true matches=0,1,2
```

### Use object shorthand for a visible field value

When a matching binding is visible, field completion inserts the shorthand form.

```ds main.ds
struct Rectangle {
    width: int32;
    height: int32;
}

const height: int32 = 20;
const rectangle: Rectangle = {
    width: 10,
    hei
    ^^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.item label=height kind=field detail=int32 insert=height sort=5 preselect=true matches=0,1,2
```

## Call Arguments

### Prefer the expected argument type

Candidates matching the active parameter type rank ahead of lexical peers.

```ds main.ds
function consume(value: int32): void {}

const candidateAlpha: string = "";
const candidateZulu: int32 = 1;

consume(candidate);
        ^^^^^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=candidateZulu kind=constant detail=int32 sort=10 preselect=true matches=0,1,2,3,4,5,6,7,8
@completion.item label=candidateAlpha kind=constant detail=string sort=10 matches=0,1,2,3,4,5,6,7,8
```

### Complete an unused named argument

Named argument completion inserts the exact remaining parameter name.

```ds main.ds
function greet(name: string, greeting: string): void {}

greet(name: "Ada", gre);
                   ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=greeting kind=field detail=string insert="greeting: ${1}" sort=5 snippet=true preselect=true matches=0,1,2
```

## Imports

### Complete a named import

Import clause completion reads the target module exports.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { gre } from "./library.ds";
         ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=greet kind=function sort=10 preselect=true matches=0,1,2
```

### Exclude an already imported name

Named import completion omits exports already present in the same clause.

```ds library.ds
export function greet(): void {}
export function grow(): void {}
```

```ds main.ds
import { greet, gr } from "./library.ds";
                ^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=grow kind=function sort=10 preselect=true matches=0,1
```

### Complete a type declaration in an import

A plain import exposes declarations from their original symbol spaces.

```ds library.ds
export type Options = {
    enabled: boolean,
};

export function open(): void {}
```

```ds main.ds
import { Opt } from "./library.ds";
         ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=Options kind=type_parameter sort=10 preselect=true matches=0,1,2
```

### Complete a re-exported name

Import completion exposes the name exported by the target module.

```ds model.ds
export function createPacket(): void {}
```

```ds library.ds
export { createPacket as packet } from "./model.ds";
```

```ds main.ds
import { pack } from "./library.ds";
         ^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=packet kind=function detail="() => void" sort=10 preselect=true matches=0,1,2,3
```

## Auto Imports

### Complete an exported function with an import edit

Program completion returns the symbol and its exact import patch together.

```ds library.ds
export function greet(): void {}
```

```ds main.ds

^ insertion
function main(): void {
    gre;
    ^^^ prefix
}
```

```query completion main.ds#prefix@end include_imports=true
@completion.item label=greet kind=function detail="Auto import from ./library" sort=100 sort_text=0:1:0:0002:0000:0009:./library:greet preselect=true auto_import=true matches=0,1,2
@completion.edit item=0 range=main.ds#insertion text="import { greet } from \"./library\";\n"
```

### Auto-import a type declaration

A type completion inserts a plain import that preserves the declaration's symbol space.

```ds library.ds
export type Widget = {
    value: string,
};
```

```ds main.ds

^ insertion
type Alias = Wid;
             ^^^ prefix
```

```query completion main.ds#prefix@end include_imports=true
@completion.item label=Widget kind=type_parameter detail="Auto import from ./library" sort=100 sort_text=0:0:0:0002:0000:0009:./library:Widget preselect=true auto_import=true matches=0,1,2
@completion.edit item=0 range=main.ds#insertion text="import { Widget } from \"./library\";\n"
```

### Auto-import a default declaration

A default export produces a default import rather than a named import.

```ds library.ds
export default function greet(name: string): string {
    return name;
}
```

```ds main.ds

^ insertion
const message = gre;
                ^^^ prefix
```

```query completion main.ds#prefix@end include_imports=true
@completion.item label=greet kind=function detail="Auto import from ./library" sort=100 sort_text=0:1:0:0002:0000:0009:./library:greet preselect=true auto_import=true matches=0,1,2
@completion.edit item=0 range=main.ds#insertion text="import greet from \"./library\";\n"
```

### Omit an import for a visible name

A visible binding wins without a redundant import candidate.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
function greet(): void {}

gre
^^^ prefix
```

```query completion main.ds#prefix@end include_imports=true
@completion.item label=greet kind=function insert="greet()$0" sort=10 snippet=true preselect=true matches=0,1,2
```

## Empty Results

### Return no completion for an ordinary string

Completion remains suppressed inside ordinary string contents.

```ds main.ds
const value = "gre";
               ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.none
```

### Return no completion inside a comment

Comments never become expression or statement completion positions.

```ds main.ds
// describe the gre value
                ^^^ prefix
const value = 1;
```

```query completion main.ds#prefix@end
@completion.none
```

### Exclude type declarations from a value position

Value completion does not cross the type namespace.

```ds main.ds
type Greeting = string;

const value = Gree;
              ^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.none
```

## Import Paths

### Complete relative modules and folders

Import path completion lists matching repository entries.

```ds utilities/helpers.ds
export function help(): void {}
```

```ds user.ds
export const user = 1;
```

```ds main.ds
import {} from "./u";
                  ^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=user kind=module sort=10 preselect=true matches=0
@completion.item label=utilities/ kind=folder sort=10 matches=0
```

### Complete a module inside a folder

Path completion continues within the selected directory.

```ds utilities/arrays.ds
export function first(): void {}
```

```ds utilities/strings.ds
export function trim(): void {}
```

```ds main.ds
import {} from "./utilities/a";
                            ^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=arrays kind=module sort=10 preselect=true matches=0
```

## Statements

### Complete a statement keyword

Statement positions include matching keywords.

```ds main.ds
function main(): void {
    ret
    ^^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=return kind=keyword sort=700 sort_text=06:return matches=0,1,2
```

### Omit statement keywords from an expression

An expression position returns values rather than unrelated statement forms.

```ds main.ds
const returnValue = 1;

const result = ret;
               ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=returnValue kind=constant detail=1 sort=10 preselect=true matches=0,1,2
```

## Primitive Types

### Complete a primitive type

Type positions include matching built-in types.

```ds main.ds
declare const value: int3;
                     ^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=int32 kind=keyword sort=20 sort_text=05:int32 matches=0,1,2,3
@completion.item label=uint32 kind=keyword sort=20 sort_text=06:uint32 matches=1,2,3,4
```
