
## Local Symbols

### Complete a local binding prefix

The matching local symbol ranks first.

```ds main.ds
const alpha = 1;
const result = alpha;
               ^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=alpha kind=constant replace=main.ds#prefix detail=1 preselect=true matches=0,1,2,3,4
```

### Replace the complete identifier

Filtering uses the text before the cursor, but accepting the item replaces the complete identifier.

```ds main.ds
const alpha = 1;
const result = alWrong;
               ^^ prefix
               ^^^^^^^ token
```

```query completion main.ds#prefix@end
@completion.item label=alpha kind=constant replace=main.ds#token detail=1 preselect=true matches=0,1
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

### Distinguish a mutable binding

Mutable bindings use their variable kind and widened type.

```ds main.ds
let mutableValue = 1;
const result = mutab;
               ^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=mutableValue kind=variable replace=main.ds#prefix detail=float64 preselect=true matches=0,1,2,3,4
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
@completion.item label=totalValue kind=value_parameter replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2,3,4,5
```

### Complete an outer binding

Nested scopes include visible bindings from their parents.

```ds main.ds
const outerValue = 1;

function read(): int32 {
    return outerV;
           ^^^^^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=outerValue kind=constant replace=main.ds#prefix detail=1 preselect=true matches=0,1,2,3,4,5
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
@completion.item label=targetValue kind=constant replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2,3,4,5,6
```

### Complete a function call

A function completion includes its signature, documentation, and call snippet.

```ds main.ds
/// Format one name.
function formatName(name: string, width: int32): string {
    return name;
}

const result = formatN;
               ^^^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=formatName kind=function replace=main.ds#prefix detail="(name: string, width: int32) => string" documentation="Format one name." insert="formatName(${1:name}, ${2:width})$0" snippet=true preselect=true matches=0,1,2,3,4,5,6
```

### Complete an imported function alias

An imported alias keeps its local name and uses the target declaration's callable type.

```ds library.ds
/// Welcome one user.
export function greet(name: string): string {
    return name;
}
```

```ds main.ds
import { greet as welcome } from "./library";

const message = wel;
                ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=welcome kind=function replace=main.ds#prefix detail="(name: string) => string" documentation="Welcome one user." insert="welcome(${1:name})$0" snippet=true preselect=true matches=0,1,2
```

### Match a camel-case prefix

Lexical matching returns the character positions used for ranking and highlighting.

```ds main.ds
const currentValue = 1;
const result = cv;
               ^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=currentValue kind=constant replace=main.ds#prefix detail=1 preselect=true matches=0,7
```

### Mark a deprecated declaration

Completion returns deprecation, documentation, and the callable edit together.

```ds main.ds
/// Use currentName.
@deprecated("use currentName")
function legacyName(): void {}

const result = legacy;
               ^^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=legacyName kind=function replace=main.ds#prefix detail="() => void" documentation="Use currentName." insert="legacyName()" preselect=true deprecated=true matches=0,1,2,3,4,5
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
@completion.item label=x kind=field replace=main.ds#member detail=int32 preselect=true matches=0
```

### Complete immediately after a dot

Member completion does not require a partial member name.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

function read(point: Point): void {
    point.;
          ^ cursor
}
```

```query completion main.ds#cursor trigger=.
@completion.item label=x kind=field replace=main.ds#cursor detail=int32
@completion.item label=y kind=field replace=main.ds#cursor detail=int32
@completion.item label=borrow kind=method replace=main.ds#cursor detail="() => WithAccess<&'a Point, A>" documentation="Borrow this value as itself." insert="borrow()"
```

### Complete a structural field

Structural field completion shows the field type.

```ds main.ds
declare const point: { x: int32; label: string };

const label = point.la;
                    ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=label kind=field replace=main.ds#prefix detail=string preselect=true matches=0,1
```

### Complete an accessor property

Getter and setter declarations form one property completion.

```ds main.ds
class Counter {
    get current(): int32 {
        return 0;
    }

    set current(next: int32) {}
}

declare const counter: Counter;
const current = counter.cur;
                        ^^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=current kind=property replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2
```

### Complete an inherited interface field

Interface completion includes members inherited from its base declarations.

```ds main.ds
interface Named {
    name: string;
}

interface User extends Named {
    id: int32;
}

declare const user: User;
const name = user.na;
                  ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=name kind=field replace=main.ds#prefix detail=string preselect=true matches=0,1
```

### Complete an instance method

Method completion includes its callable type and insertion snippet.

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
@completion.item label=read kind=method replace=main.ds#prefix detail="(index: uint64) => uint8" documentation="Read one byte." insert="read(${1:index})$0" snippet=true preselect=true matches=0,1
```

### Complete an inherited class method

A derived class exposes methods declared by its base class.

```ds main.ds
class Resource {
    close(): void {}
}

class File extends Resource {}

declare const file: File;
file.clo;
     ^^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=close kind=method replace=main.ds#prefix detail="() => void" insert="close()" preselect=true matches=0,1,2
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
value.cre;
      ^^^ instance_prefix
```

```query completion main.ds#static_prefix@end trigger=.
@completion.item label=create kind=method replace=main.ds#static_prefix detail="(size: uint64) => Buffer" insert="create(${1:size})$0" snippet=true preselect=true matches=0,1
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
@completion.item label=sum kind=method replace=main.ds#prefix detail="(left: int32, right: int32) => int32" insert="sum(${1:left}, ${2:right})$0" snippet=true preselect=true matches=0,1
```

### Complete an applied extension method

A generic extension method uses the receiver's applied type arguments.

```ds main.ds
struct Box<Value> {
    value: Value;
}

extension<Value> of Box<Value> {
    unwrap(): Value {
        return this.value;
    }
}

function read(box: Box<string>): string {
    return box.unw;
               ^^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=unwrap kind=method replace=main.ds#prefix detail="() => string" insert="unwrap()" preselect=true matches=0,1,2
```

### Complete an applicable blanket extension

A blanket extension appears when its receiver constraint is satisfied.

```ds main.ds
interface Named {}

struct User implements Named {}

extension<Value: Named> of Value {
    displayName(): string {
        return "user";
    }
}

function display(user: User): string {
    return user.dis;
                ^^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=displayName kind=method replace=main.ds#prefix detail="() => string" insert="displayName()" preselect=true matches=0,1,2
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
@completion.item label=x kind=field replace=main.ds#prefix detail=int32 preselect=true matches=0
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
@completion.item label=value kind=field replace=main.ds#prefix detail=string preselect=true matches=0,1,2
```

### Complete a constrained parameter member

A type parameter exposes members declared by its constraint.

```ds main.ds
interface Named {
    name: string;
}

function nameOf<Value: Named>(value: Value): string {
    return value.na;
                 ^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=name kind=field replace=main.ds#prefix detail=string preselect=true matches=0,1
```

### Complete a newtype backing member

A newtype exposes members selected through its backing value.

```ds main.ds
newtype User = { name: string };

function nameOf(user: User): string {
    return user.na;
                ^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=name kind=field replace=main.ds#prefix detail=string preselect=true matches=0,1
```

### Complete primitive extension members

Primitive values expose their implicit extension methods.

```ds main.ds
function isEmpty(value: string): boolean {
    return value.isE;
                 ^^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=isEmpty kind=property replace=main.ds#prefix detail=boolean documentation="Check if the string is empty." matches=0,1,2
@completion.item label=isWellFormed kind=method replace=main.ds#prefix detail="() => boolean" documentation="Return whether this string is well-formed Unicode." insert="isWellFormed()" matches=0,1,3
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
@completion.item label=label kind=field replace=main.ds#common_prefix detail=string preselect=true matches=0,1
```

```query completion main.ds#partial_prefix@end trigger=.
@completion.none
```

### Omit an unavailable union operation

A union only offers members that every possible value can use in the same way.

```ds main.ds
class Source {
    get value(): string {
        return "";
    }
}

class Sink {
    set value(next: string) {}
}

function read(target: Source | Sink): void {
    target.val;
           ^^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.none
```

### Complete intersection members

An intersection exposes members from each constituent.

```ds main.ds
interface Named {
    name: string;
}

interface Identified {
    id: int32;
}

function identify(value: Named & Identified): int32 {
    return value.i;
                 ^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=id kind=field replace=main.ds#prefix detail=int32 preselect=true matches=0
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
@completion.item label=Width kind=associated_const replace=main.ds#prefix detail=uint64 preselect=true matches=0,1
```

### Complete an associated type

A constrained type parameter exposes its associated types.

```ds main.ds
interface Collection {
    type Item;
}

function item<T: Collection>(): T.Ite;
                                  ^^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Item kind=associated_type replace=main.ds#prefix detail=T.Item preselect=true matches=0,1,2
```

### Complete a static member through a type alias

A type alias exposes static members from its target declaration.

```ds main.ds
struct Buffer {
    comptime const Width: uint = 8;
}

type BufferAlias = Buffer;
const width = BufferAlias.Wi;
                          ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Width kind=associated_const replace=main.ds#prefix detail=uint64 preselect=true matches=0,1
```

### Complete a namespace member

A namespace receiver exposes the exports of its target module.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import * as library from "./library.ds";

library.gr;
        ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=greet kind=function replace=main.ds#prefix detail="() => void" insert="greet()" preselect=true matches=0,1
```

### Complete a namespace type

A namespace path exposes exported type declarations in type positions.

```ds library.ds
export struct Packet {}
```

```ds main.ds
import * as library from "./library.ds";

declare const packet: library.Pac;
                              ^^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Packet kind=struct replace=main.ds#prefix preselect=true matches=0,1,2
```

## Implementations

### [ignored] Complete a required method

A class body can insert the entire missing interface method.

```ds main.ds
interface Service {
    run(value: int32): void;
    stop(): void;
}

class Application implements Service {
    run(value: int32): void {}

    st
    ^^ prefix
}
```

```query completion main.ds#prefix@end
@completion.item label=stop kind=method replace=main.ds#prefix detail="Implement Service.stop" insert="stop(): void {\n    $0\n}" snippet=true preselect=true matches=0,1
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
@completion.item label=Point kind=struct replace=main.ds#prefix preselect=true matches=0,1,2
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
@completion.item label=Value kind=type_parameter replace=main.ds#prefix preselect=true matches=0,1,2
```

### Complete an imported type through a re-export

Type completion preserves the declaration kind through module aliases.

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
@completion.item label=Packet kind=struct replace=main.ds#prefix preselect=true matches=0,1,2,3
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
@completion.item label=Red kind=enum_member replace=main.ds#prefix detail=Color.Red preselect=true matches=0
```

### Complete a tagged variant

A tagged case completion carries its payload constructor and result type.

```ds main.ds
@derive(Tagged)
newtype Status =
    | { type: "ok"; value: string }
    | { type: "error"; error: int32 };

const status = Status.Ok;
                      ^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=Ok kind=constructor replace=main.ds#prefix detail="({ value: string }) => Status.Ok" insert="Ok({ value: ${1} })$0" snippet=true preselect=true matches=0,1
```

### Complete a tagged discriminator

Tagged values expose their discriminator with every possible tag.

```ds main.ds
@derive(Tagged)
newtype Status =
    | { type: "ok"; value: string }
    | { type: "error"; error: int32 };

function statusType(status: Status): string {
    return status.ty;
                  ^^ prefix
}
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=type kind=field replace=main.ds#prefix detail="\"ok\" | \"error\"" preselect=true matches=0,1
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
                   ^^^^^^ token
```

```query completion main.ds#prefix@end
@completion.item label=Widget kind=class replace=main.ds#token detail="new (name: string) => this" insert="Widget(${1:name})$0" snippet=true preselect=true matches=0,1,2
```

### Complete a struct expression

A struct value completion can insert every required field.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

const result = Poi;
               ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=Point kind=struct replace=main.ds#prefix detail=Point insert="Point { x: ${1}, y: ${2} }$0" snippet=true preselect=true matches=0,1,2
```

### Complete a newtype constructor

A newtype completion includes its constructor signature and call snippet.

```ds main.ds
newtype UserId = string;

const result = UserI;
               ^^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=UserId kind=constructor replace=main.ds#prefix detail="(string) => UserId" insert="UserId(${1})$0" snippet=true preselect=true matches=0,1,2,3,4
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
@completion.item label=height kind=field replace=main.ds#prefix detail=int32 insert="height: ${1}" snippet=true preselect=true matches=0,1,2
```

### Substitute a generic field type

A generic object field uses the applied type argument.

```ds main.ds
struct Box<Value> {
    value: Value;
}

const box: Box<string> = {
    val
    ^^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.item label=value kind=field replace=main.ds#prefix detail=string insert="value: ${1}" snippet=true preselect=true matches=0,1,2
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
@completion.item label=street kind=field replace=main.ds#prefix detail=string insert="street: ${1}" snippet=true preselect=true matches=0,1,2
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
@completion.item label=height kind=field replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2
```

### Complete a visible shorthand without a contextual type

An object literal can use any visible value as a shorthand property.

```ds main.ds
const height: int32 = 20;
const rectangle = {
    hei
    ^^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.item label=height kind=field replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2
```

### Keep computed fields distinct

A computed field does not hide a different named field.

```ds main.ds
declare const token: unique symbol;

type Payload = {
    [token]: string;
    name: string;
};

const payload: Payload = {
    [token]: "value",
    na
    ^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.item label=name kind=field replace=main.ds#prefix detail=string insert="name: ${1}" snippet=true preselect=true matches=0,1
```

### Omit a field supplied by a spread

A spread supplies its fields to the surrounding object.

```ds main.ds
struct Rectangle {
    width: int32;
    height: int32;
}

const partial = { width: 10 };
const rectangle: Rectangle = {
    ...partial,
    wid
    ^^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.none
```

### Omit a field supplied by a nominal spread

A nominal spread supplies its selected instance fields.

```ds main.ds
struct PartialRectangle {
    width: int32;
}

struct Rectangle {
    width: int32;
    height: int32;
}

const partial: PartialRectangle = { width: 10 };
const rectangle: Rectangle = {
    ...partial,
    wid
    ^^^ prefix
};
```

```query completion main.ds#prefix@end
@completion.none
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
@completion.item label=candidateZulu kind=constant replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2,3,4,5,6,7,8
@completion.item label=candidateAlpha kind=constant replace=main.ds#prefix detail=string matches=0,1,2,3,4,5,6,7,8
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
@completion.item label=greet kind=function replace=main.ds#prefix detail="() => void" preselect=true matches=0,1,2
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
@completion.item label=grow kind=function replace=main.ds#prefix detail="() => void" preselect=true matches=0,1
```

### Complete a type declaration in an import

A named import exposes declarations from their original symbol spaces.

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
@completion.item label=Options kind=type_alias replace=main.ds#prefix preselect=true matches=0,1,2
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
@completion.item label=packet kind=function replace=main.ds#prefix detail="() => void" preselect=true matches=0,1,2,3
```

## Auto Imports

### Complete an exported function with an import edit

Auto import completion returns the symbol and import patch together.

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

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=greet kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="greet()" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="import { greet } from \"./library\";\n"
```

### Refresh an incomplete completion list

A retriggered request continues the candidate family returned by the incomplete list.

```ds library.ds
export function target(): void {}
```

```ds main.ds

^ insertion
function main(): void {
    t;
    ^ prefix
}
```

```query completion main.ds#prefix@end trigger=incomplete include_auto_imports=true
@completion.item label=target kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="target()" preselect=true auto_import=true matches=0
@completion.additional_edit item=0 range=main.ds#insertion text="import { target } from \"./library\";\n"
```

### Report a truncated completion list

A truncated short-prefix list reports that more matching candidates are available.

```ds library.ds
export type x00 = int32;
export type x01 = int32;
export type x02 = int32;
export type x03 = int32;
export type x04 = int32;
export type x05 = int32;
export type x06 = int32;
export type x07 = int32;
export type x08 = int32;
export type x09 = int32;
export type x10 = int32;
export type x11 = int32;
export type x12 = int32;
export type x13 = int32;
export type x14 = int32;
export type x15 = int32;
export type x16 = int32;
export type x17 = int32;
export type x18 = int32;
export type x19 = int32;
export type x20 = int32;
export type x21 = int32;
export type x22 = int32;
export type x23 = int32;
export type x24 = int32;
export type x25 = int32;
export type x26 = int32;
export type x27 = int32;
export type x28 = int32;
export type x29 = int32;
export type x30 = int32;
export type x31 = int32;
export type x32 = int32;
export type x33 = int32;
export type x34 = int32;
export type x35 = int32;
export type x36 = int32;
export type x37 = int32;
export type x38 = int32;
export type x39 = int32;
export type x40 = int32;
export type x41 = int32;
export type x42 = int32;
export type x43 = int32;
export type x44 = int32;
export type x45 = int32;
export type x46 = int32;
export type x47 = int32;
export type x48 = int32;
export type x49 = int32;
export type x50 = int32;
```

```ds main.ds

^ insertion
type Selected = x;
                ^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.list incomplete=true
@completion.item label=x00 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=0 range=main.ds#insertion text="import { x00 } from \"./library\";\n"
@completion.item label=x01 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=1 range=main.ds#insertion text="import { x01 } from \"./library\";\n"
@completion.item label=x02 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=2 range=main.ds#insertion text="import { x02 } from \"./library\";\n"
@completion.item label=x03 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=3 range=main.ds#insertion text="import { x03 } from \"./library\";\n"
@completion.item label=x04 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=4 range=main.ds#insertion text="import { x04 } from \"./library\";\n"
@completion.item label=x05 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=5 range=main.ds#insertion text="import { x05 } from \"./library\";\n"
@completion.item label=x06 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=6 range=main.ds#insertion text="import { x06 } from \"./library\";\n"
@completion.item label=x07 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=7 range=main.ds#insertion text="import { x07 } from \"./library\";\n"
@completion.item label=x08 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=8 range=main.ds#insertion text="import { x08 } from \"./library\";\n"
@completion.item label=x09 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=9 range=main.ds#insertion text="import { x09 } from \"./library\";\n"
@completion.item label=x10 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=10 range=main.ds#insertion text="import { x10 } from \"./library\";\n"
@completion.item label=x11 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=11 range=main.ds#insertion text="import { x11 } from \"./library\";\n"
@completion.item label=x12 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=12 range=main.ds#insertion text="import { x12 } from \"./library\";\n"
@completion.item label=x13 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=13 range=main.ds#insertion text="import { x13 } from \"./library\";\n"
@completion.item label=x14 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=14 range=main.ds#insertion text="import { x14 } from \"./library\";\n"
@completion.item label=x15 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=15 range=main.ds#insertion text="import { x15 } from \"./library\";\n"
@completion.item label=x16 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=16 range=main.ds#insertion text="import { x16 } from \"./library\";\n"
@completion.item label=x17 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=17 range=main.ds#insertion text="import { x17 } from \"./library\";\n"
@completion.item label=x18 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=18 range=main.ds#insertion text="import { x18 } from \"./library\";\n"
@completion.item label=x19 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=19 range=main.ds#insertion text="import { x19 } from \"./library\";\n"
@completion.item label=x20 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=20 range=main.ds#insertion text="import { x20 } from \"./library\";\n"
@completion.item label=x21 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=21 range=main.ds#insertion text="import { x21 } from \"./library\";\n"
@completion.item label=x22 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=22 range=main.ds#insertion text="import { x22 } from \"./library\";\n"
@completion.item label=x23 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=23 range=main.ds#insertion text="import { x23 } from \"./library\";\n"
@completion.item label=x24 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=24 range=main.ds#insertion text="import { x24 } from \"./library\";\n"
@completion.item label=x25 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=25 range=main.ds#insertion text="import { x25 } from \"./library\";\n"
@completion.item label=x26 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=26 range=main.ds#insertion text="import { x26 } from \"./library\";\n"
@completion.item label=x27 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=27 range=main.ds#insertion text="import { x27 } from \"./library\";\n"
@completion.item label=x28 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=28 range=main.ds#insertion text="import { x28 } from \"./library\";\n"
@completion.item label=x29 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=29 range=main.ds#insertion text="import { x29 } from \"./library\";\n"
@completion.item label=x30 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=30 range=main.ds#insertion text="import { x30 } from \"./library\";\n"
@completion.item label=x31 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=31 range=main.ds#insertion text="import { x31 } from \"./library\";\n"
@completion.item label=x32 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=32 range=main.ds#insertion text="import { x32 } from \"./library\";\n"
@completion.item label=x33 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=33 range=main.ds#insertion text="import { x33 } from \"./library\";\n"
@completion.item label=x34 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=34 range=main.ds#insertion text="import { x34 } from \"./library\";\n"
@completion.item label=x35 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=35 range=main.ds#insertion text="import { x35 } from \"./library\";\n"
@completion.item label=x36 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=36 range=main.ds#insertion text="import { x36 } from \"./library\";\n"
@completion.item label=x37 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=37 range=main.ds#insertion text="import { x37 } from \"./library\";\n"
@completion.item label=x38 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=38 range=main.ds#insertion text="import { x38 } from \"./library\";\n"
@completion.item label=x39 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=39 range=main.ds#insertion text="import { x39 } from \"./library\";\n"
@completion.item label=x40 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=40 range=main.ds#insertion text="import { x40 } from \"./library\";\n"
@completion.item label=x41 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=41 range=main.ds#insertion text="import { x41 } from \"./library\";\n"
@completion.item label=x42 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=42 range=main.ds#insertion text="import { x42 } from \"./library\";\n"
@completion.item label=x43 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=43 range=main.ds#insertion text="import { x43 } from \"./library\";\n"
@completion.item label=x44 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=44 range=main.ds#insertion text="import { x44 } from \"./library\";\n"
@completion.item label=x45 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=45 range=main.ds#insertion text="import { x45 } from \"./library\";\n"
@completion.item label=x46 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=46 range=main.ds#insertion text="import { x46 } from \"./library\";\n"
@completion.item label=x47 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=47 range=main.ds#insertion text="import { x47 } from \"./library\";\n"
@completion.item label=x48 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=48 range=main.ds#insertion text="import { x48 } from \"./library\";\n"
@completion.item label=x49 kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" auto_import=true matches=0
@completion.additional_edit item=49 range=main.ds#insertion text="import { x49 } from \"./library\";\n"
```

### Preserve an extension for an ambiguous relative path

An explicit extension keeps the target module unambiguous.

```ds library.ds
export function greet(): void {}
```

```ds library.d.ds
export type LibraryDeclaration = string;
```

```ds main.ds

^ insertion
gre
^^^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=greet kind=function replace=main.ds#prefix detail="Auto import from ./library.ds" insert="greet()" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="import { greet } from \"./library.ds\";\n"
```

### Auto-import a type declaration

A type completion inserts a named import that preserves the declaration's symbol space.

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

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=Widget kind=type_alias replace=main.ds#prefix detail="Auto import from ./library" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="import { Widget } from \"./library\";\n"
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

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=greet kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="greet(${1:name})$0" snippet=true preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="import greet from \"./library\";\n"
```

### Auto-import an overload family once

An exported overload family produces one completion and one import edit.

```ds library.ds
export function parse(value: int32): int32 {
    return value;
}

export function parse(value: string): string {
    return value;
}
```

```ds main.ds

^ insertion
const value = par;
              ^^^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=parse kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="parse(${1:value})$0" snippet=true preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="import { parse } from \"./library\";\n"
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

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=greet kind=function replace=main.ds#prefix detail="() => void" insert="greet()" preselect=true matches=0,1,2
```

### Auto-import a public package export

Package completion uses the active dependency name and public export pattern.

```json destack.json
{
    "workspace": {
        "packages": ["packages/*"]
    }
}
```

```json packages/app/destack.json
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.ds"]
        }
    },
    "defaultTarget": "default"
}
```

```json packages/ui/destack.json
{
    "name": "@acme/ui",
    "exports": {
        "./*": {
            "kind": "module",
            "path": "src/*.ds"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.ds"]
        }
    },
    "defaultTarget": "default"
}
```

```ds packages/ui/src/button.ds
export struct Button {}
```

```ds packages/app/main.ds

^ insertion
const value = But;
              ^^^ prefix
```

```query completion packages/app/main.ds#prefix@end include_auto_imports=true
@completion.item label=Button kind=struct replace=packages/app/main.ds#prefix detail="Auto import from @acme/ui/button" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=packages/app/main.ds#insertion text="import { Button } from \"@acme/ui/button\";\n"
```

### Omit an ambiguous package export

An extensionless export that selects several modules is not a valid auto import.

```json destack.json
{
    "workspace": {
        "packages": ["packages/*"]
    }
}
```

```json packages/app/destack.json
{
    "name": "app",
    "dependencies": {
        "@acme/ui": {
            "source": "workspace"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.ds"]
        }
    },
    "defaultTarget": "default"
}
```

```json packages/ui/destack.json
{
    "name": "@acme/ui",
    "exports": {
        "./button": {
            "kind": "module",
            "path": "src/button"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.ds"]
        }
    },
    "defaultTarget": "default"
}
```

```ds packages/ui/src/button.ds
export struct Button {}
```

```ds packages/ui/src/button.d.ds
export type ButtonDeclaration = string;
```

```ds packages/app/main.ds
But
^^^ prefix
```

```query completion packages/app/main.ds#prefix@end include_auto_imports=true
@completion.none
```

### Auto-import a package root export

A root export produces the dependency package name without a subpath.

```json destack.json
{
    "workspace": {
        "packages": ["packages/*"]
    }
}
```

```json packages/app/destack.json
{
    "name": "app",
    "dependencies": {
        "@acme/theme": {
            "source": "workspace"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.ds"]
        }
    },
    "defaultTarget": "default"
}
```

```json packages/theme/destack.json
{
    "name": "@acme/theme",
    "exports": {
        ".": {
            "kind": "module",
            "path": "src/main.ds"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.ds"]
        }
    },
    "defaultTarget": "default"
}
```

```ds packages/theme/src/main.ds
export struct Theme {}
```

```ds packages/app/main.ds

^ insertion
const value = The;
              ^^^ prefix
```

```query completion packages/app/main.ds#prefix@end include_auto_imports=true
@completion.item label=Theme kind=struct replace=packages/app/main.ds#prefix detail="Auto import from @acme/theme" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=packages/app/main.ds#insertion text="import { Theme } from \"@acme/theme\";\n"
```

### Add a named binding to an existing import

An auto import extends the matching named import declaration.

```ds library.ds
export function alpha(): void {}
export function beta(): void {}
```

```ds main.ds
import { alpha } from "./library";
              ^ insertion

const value = bet;
              ^^^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=beta kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="beta()" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text=", beta"
```

### Add a named binding beside a default import

An auto import adds a named clause after the existing default binding.

```ds library.ds
export default function build(): void {}
export function beta(): void {}
```

```ds main.ds
import build from "./library";
            ^ default_end

const value = bet;
              ^^^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=beta kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="beta()" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#default_end text=", { beta }"
```

### Add a default binding beside named imports

An auto import adds the default binding before the existing named clause.

```ds library.ds
export default function build(): void {}
export function value(): void {}
```

```ds main.ds
import { value } from "./library";
       ^ insertion

const result = bui;
               ^^^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=build kind=function replace=main.ds#prefix detail="Auto import from ./library" insert="build()" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="build, "
```

### Insert a new import after an earlier import

An auto import preserves a complete line between consecutive declarations.

```ds alpha.ds
export function alpha(): void {}
```

```ds beta.ds
export function beta(): void {}
```

```ds main.ds
import { alpha } from "./alpha";
                                ^ insertion

const value = bet;
              ^^^ prefix
```

```query completion main.ds#prefix@end include_auto_imports=true
@completion.item label=beta kind=function replace=main.ds#prefix detail="Auto import from ./beta" insert="beta()" preselect=true auto_import=true matches=0,1,2
@completion.additional_edit item=0 range=main.ds#insertion text="\nimport { beta } from \"./beta\";"
```

## Empty Results

### Return no completion for a string literal

Completion remains suppressed inside string literal contents.

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

Import path completion lists matching modules in the current package.

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
@completion.item label=user kind=module replace=main.ds#prefix matches=0
@completion.item label=utilities/ kind=folder replace=main.ds#prefix matches=0
```

### Complete a module inside a folder

Path completion continues within the requested directory.

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
@completion.item label=arrays kind=module replace=main.ds#prefix preselect=true matches=0
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
@completion.item label=return kind=keyword replace=main.ds#prefix preselect=true matches=0,1,2
```

### Omit statement keywords from an expression

An expression position returns values rather than unrelated statement forms.

```ds main.ds
const returnValue = 1;

const result = ret;
               ^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=returnValue kind=constant replace=main.ds#prefix detail=1 preselect=true matches=0,1,2
```

## Primitive Types

### Complete a primitive type

Type positions include matching built-in types.

```ds main.ds
declare const value: int3;
                     ^^^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=int32 kind=builtin_type replace=main.ds#prefix matches=0,1,2,3
@completion.item label=uint32 kind=builtin_type replace=main.ds#prefix matches=1,2,3,4
```

## Source changes

### Update visible symbols after a declaration changes

Completion reflects the declarations in the selected revision.

```ds main.ds
const alpha = 1;
const result = al;
               ^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=alpha kind=constant replace=main.ds#prefix detail=1 preselect=true matches=0,1
```

```ds main.ds change
const alpine = 2;
const result = al;
               ^^ prefix
```

```query completion main.ds#prefix@end
@completion.item label=alpine kind=constant replace=main.ds#prefix detail=2 preselect=true matches=0,1
```

### Update members after successive source changes

Member completion uses the receiver selected in each revision.

```ds main.ds
struct Box {
    value: string;
}

declare const box: Box;
const selected = box.val;
                     ^^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=value kind=field replace=main.ds#prefix detail=string preselect=true matches=0,1,2
```

```ds main.ds change
struct Box {
    count: int32;
}

declare const box: Box;
const selected = box.cou;
                     ^^^ prefix
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=count kind=field replace=main.ds#prefix detail=int32 preselect=true matches=0,1,2
```

```diff main.ds
@@ -1,3 +1,3 @@
 struct Box {
-    count: int32;
+    count: boolean;
 }
```

```query completion main.ds#prefix@end trigger=.
@completion.item label=count kind=field replace=main.ds#prefix detail=boolean preselect=true matches=0,1,2
```
