
## Local Symbols

### Complete a local binding prefix

The matching local symbol ranks first.

```tspp main.tspp
const alpha = 1;
const result = alpha;
               ^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=alpha kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4
```

### Replace the complete identifier

Filtering uses the text before the cursor, but accepting the item replaces the complete identifier.

```tspp main.tspp
const alpha = 1;
const result = alphaWrong;
               ^^^^^ prefix
               ^^^^^^^^^^ token
```

```query completion main.tspp#prefix@end
@completion.item label=alpha kind=constant replace=main.tspp#token suffix=": 1" matches=0,1,2,3,4
```

### Exclude a later local declaration

A declaration is not visible before its lexical declaration point.

```tspp main.tspp
function read(): void {
    futureScope
    ^^^^^^^^^^^ prefix

    const futureScopeValue = 1;
}
```

```query completion main.tspp#prefix@end
@completion.none
```

### Exclude the binding being initialized

An initializer cannot use the binding introduced by its own declarator.

```tspp main.tspp
const target: string = "outer";

function read(): string {
    const target = target;
                   ^^^^^^ prefix
    return target;
}
```

```query completion main.tspp#prefix@end
@completion.item label=target kind=constant replace=main.tspp#prefix suffix=": string" matches=0,1,2,3,4,5
```

### Exclude the current destructuring pattern

Bindings introduced by a declarator are unavailable throughout its initializer.

```tspp main.tspp
const target = 1;
const { targetField, source: targetAlias } = target;
                                             ^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=target kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4,5
```

### Retain an earlier destructuring binding

A default value can use bindings evaluated earlier in the same pattern, but not its own binding.

```tspp main.tspp
declare const source: { targetValue?: int32; targetField?: int32 };
const { targetValue = 1, targetField = targetValue } = source;
                                       ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=targetValue kind=constant replace=main.tspp#prefix suffix=": int32" matches=0,1,2,3,4,5,6,7,8,9,10
```

### Distinguish a mutable binding

Mutable bindings use their variable kind and widened type.

```tspp main.tspp
let mutableValue = 1;
const result = mutableValue;
               ^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=mutableValue kind=variable replace=main.tspp#prefix suffix=": int64" matches=0,1,2,3,4,5,6,7,8,9,10,11
```

### Complete a function parameter

Parameters remain visible throughout their function body.

```tspp main.tspp
function calculate(totalValue: int32): int32 {
    return totalV;
           ^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end
@completion.item label=totalValue kind=value_parameter replace=main.tspp#prefix suffix=": int32" matches=0,1,2,3,4,5
```

### Complete an outer binding

Nested scopes include visible bindings from their parents.

```tspp main.tspp
const outerValue = 1;

function read(): int32 {
    return outerV;
           ^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end
@completion.item label=outerValue kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4,5
```

### Prefer the nearest shadowing declaration

One visible name produces one item using the innermost declaration.

```tspp main.tspp
const targetValue: string = "";

function read(): int32 {
    const targetValue: int32 = 1;
    return targetV;
           ^^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end
@completion.item label=targetValue kind=constant replace=main.tspp#prefix suffix=": int32" matches=0,1,2,3,4,5,6
```

### Shadow a type declaration with a local value

A local value hides the outer declaration in type annotations too.

```tspp main.tspp
struct FixtureTarget {
    x: int32;
}

function read(): void {
    const FixtureTarget = 1;

    let value: FixtureTar;
               ^^^^^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end
@completion.none
```

### Complete a function call

A function completion includes its signature, documentation, and call snippet.

```tspp main.tspp
/// Format one name.
/// @param name - The name to format.
/// @param width - The requested width.
/// @example
/// ```tspp
/// formatName("Ada", 8);
/// ```
function formatName(name: string, width: int32): string {
    return name;
}

const result = formatN;
               ^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=formatName kind=function replace=main.tspp#prefix suffix="(name: string, width: int32): string" insert="formatName(${1:name}, ${2:width})$0" snippet=true matches=0,1,2,3,4,5,6
```

### Complete dollar-prefixed names

Call completion preserves dollar signs in function and parameter names.

```tspp main.tspp
function fixture$read($value: int32): int32 { return $value; }

const result = fixture$rea;
               ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label="fixture$read" kind=function replace=main.tspp#prefix suffix="($value: int32): int32" insert="fixture\\$read(${1:\\$value})$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10
```

### Complete destructured parameters

Call completion preserves the complete destructuring pattern in its parameter placeholder.

```tspp main.tspp
function fixtureRead({ value }: { value: int32 }): int32 { return value; }

const result = fixtureRea;
               ^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=fixtureRead kind=function replace=main.tspp#prefix suffix="({ value }: { value: int32 }): int32" insert="fixtureRead(${1:{ value \\}})$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9
```

### Complete an imported function alias

An imported alias keeps its local name and uses the target declaration's callable type.

```tspp library.tspp
/// Welcome one user.
export function greet(name: string): string {
    return name;
}
```

```tspp main.tspp
import { greet as welcome } from "./library";

const message = wel;
                ^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=welcome kind=function replace=main.tspp#prefix suffix="(name: string): string" insert="welcome(${1:name})$0" snippet=true matches=0,1,2
```

### Match a camel-case prefix

Lexical matching returns the character positions used for ranking and highlighting.

```tspp main.tspp
const fixtureCurrentValue = 1;
const result = fCV;
               ^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=fixtureCurrentValue kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,7,14
```

### Match a Unicode identifier

Match positions count characters rather than UTF-8 bytes.

```tspp main.tspp
const caféValue = 1;
const result = caféV;
               ^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label="caféValue" kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4
```

### Mark a deprecated declaration

Completion returns deprecation, documentation, and the callable edit together.

```tspp main.tspp
/// Use currentName.
@deprecated("use currentName")
function legacyName(): void {}

const result = legacy;
               ^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=legacyName kind=function replace=main.tspp#prefix suffix="(): void" insert="legacyName()" deprecated=true matches=0,1,2,3,4,5
```

### Complete visible symbols from current declarations

Completion reflects the declarations after each edit.

```tspp main.tspp
const localRevisionAlpha = 1;
const result = localRevision;
               ^^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=localRevisionAlpha kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4,5,6,7,8,9,10,11,12
```

```tspp main.tspp change
const localRevisionAlpine = 2;
const result = localRevision;
               ^^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=localRevisionAlpine kind=constant replace=main.tspp#prefix suffix=": 2" matches=0,1,2,3,4,5,6,7,8,9,10,11,12
```

## Global Symbols

### Complete a profile global

Profile globals participate in ordinary value completion.

```json destack.json
{
  "name": "@test/query",
  "compiler": {
    "globals": ["global.tspp"]
  },
  "targets": {
    "default": {
      "include": ["**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
```

```tspp library.tspp
export struct Context {}

/// Return the current execution context.
export declare function currentContext(): Context;
```

```tspp global.tspp
global {
    export { Context, currentContext } from "./library.tspp";
}
```

```tspp main.tspp
const context = currentContex;
                ^^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=currentContext kind=function replace=main.tspp#prefix suffix="(): Context" insert="currentContext()" matches=0,1,2,3,4,5,6,7,8,9,10,11,12
```

## Members

### Complete struct fields

Member completion lists the receiver's fields.

```tspp main.tspp
struct Point {
    x: int32;
    y: int32;
}

function read(point: Point): int32 {
    return point.x;
                 ^ member
}
```

```query completion main.tspp#member@end trigger=.
@completion.item label=x kind=field replace=main.tspp#member suffix=": int32" matches=0
```

### Complete immediately after a dot

Member completion does not require a partial member name.

```tspp main.tspp
struct Point {
    x: int32;
    y: int32;
}

function read(point: Point): void {
    point.;
          ^ cursor
}
```

```query completion main.tspp#cursor trigger=.
@completion.item label=x kind=field replace=main.tspp#cursor suffix=": int32"
@completion.item label=y kind=field replace=main.tspp#cursor suffix=": int32"
@completion.item label=borrow kind=method replace=main.tspp#cursor suffix="(): Borrowed<Point, R, A>" description="as Borrow<Point, A>" insert="borrow()"
@completion.item label=into kind=method replace=main.tspp#cursor suffix="(): U" description="as Into<U>" insert="into()"
@completion.item label=tryInto kind=method replace=main.tspp#cursor suffix="(): Result<U, U.Error>" description="as TryInto<U>" insert="tryInto()"
```

### Complete constructor receiver members while typing

Member completion uses the enclosing class while typing inside its constructor.

```tspp main.tspp
class User {
    name: string;
    age: uint;

    constructor(name: string, age: uint) {
        this.name = name;
        this.age = age;
    }
}
```

```tspp main.tspp type
class User {
    name: string;
    age: uint;

    constructor(name: string, age: uint) {
        this.name = name;
        this.age = age;
        this.
             ^ cursor
    }
}
```

```query completion main.tspp#cursor trigger=.
@completion.item label=name kind=field replace=main.tspp#cursor suffix=": string"
@completion.item label=age kind=field replace=main.tspp#cursor suffix=": uint64"
@completion.item label=borrow kind=method replace=main.tspp#cursor suffix="(): Borrowed<User, R, A>" description="as Borrow<User, A>" insert="borrow()"
@completion.item label=into kind=method replace=main.tspp#cursor suffix="(): U" description="as Into<U>" insert="into()"
@completion.item label=tryInto kind=method replace=main.tspp#cursor suffix="(): Result<U, U.Error>" description="as TryInto<U>" insert="tryInto()"
```

### Complete through generic borrow access

Member completion traverses a borrowed receiver with generic access.

```tspp main.tspp
struct Box<Value> {
    value: Value;
}

extension<Value, const A: Access = "readonly"> of Box<Value> {
    read(this: WithAccess<&Box<Value>, A>): Value {
        return this.val;
                    ^^^ prefix
    }
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=value kind=field replace=main.tspp#prefix suffix=": Value" matches=0,1,2
```

### Complete a structural field

Structural field completion shows the field type.

```tspp main.tspp
declare const point: { x: int32; label: string };

const label = point.la;
                    ^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=label kind=field replace=main.tspp#prefix suffix=": string" matches=0,1
```

### Complete an accessor property

Getter and setter declarations form one property completion.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=current kind=property replace=main.tspp#prefix suffix=": int32" matches=0,1,2
```

### Complete an inherited interface field

Interface completion includes members inherited from its base declarations.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=name kind=field replace=main.tspp#prefix suffix=": string" matches=0,1
```

### Complete an instance method

Method completion includes its callable type and insertion snippet.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=read kind=method replace=main.tspp#prefix suffix="(index: uint64): uint8" insert="read(${1:index})$0" snippet=true matches=0,1
```

### Complete an inherited class method

A derived class exposes methods declared by its base class.

```tspp main.tspp
class Resource {
    close(): void {}
}

class File extends Resource {}

declare const file: File;
file.clo;
     ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=close kind=method replace=main.tspp#prefix suffix="(): void" insert="close()" matches=0,1,2
```

### Separate static and instance members

Type receivers expose static members and value receivers expose instance members.

```tspp main.tspp
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

```query completion main.tspp#static_prefix@end trigger=.
@completion.item label=create kind=method replace=main.tspp#static_prefix suffix="(size: uint64): Buffer" insert="create(${1:size})$0" snippet=true matches=0,1
```

```query completion main.tspp#instance_prefix@end trigger=.
@completion.none
```

### Complete an extension method

Member completion includes extension methods for the receiver type.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=sum kind=method replace=main.tspp#prefix suffix="(left: int32, right: int32): int32" insert="sum(${1:left}, ${2:right})$0" snippet=true matches=0,1
```

### Complete an applied extension method

A generic extension method uses the receiver's applied type arguments.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=unwrap kind=method replace=main.tspp#prefix suffix="(): string" insert="unwrap()" matches=0,1,2
```

### Complete an applicable blanket extension

A blanket extension appears when its receiver constraint is satisfied.

```tspp main.tspp
newtype interface Named {}

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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=displayName kind=method replace=main.tspp#prefix suffix="(): string" insert="displayName()" matches=0,1,2
```

### Omit an inapplicable blanket extension

A constrained blanket extension does not appear for a receiver outside its bound.

```tspp main.tspp
newtype interface Named {}

struct User {}

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

```query completion main.tspp#prefix@end trigger=.
@completion.none
```

### Complete an optional-chain member

Optional chaining completes the same members as direct access.

```tspp main.tspp
struct Point {
    x: int32;
}

function read(point: Point | undefined): int32 | undefined {
    return point?.x;
                  ^ prefix
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=x kind=field replace=main.tspp#prefix suffix=": int32" matches=0
```

### Complete a generic field

A generic field uses the receiver's applied type arguments.

```tspp main.tspp
struct Box<Value> {
    value: Value;
}

function read(box: Box<string>): string {
    return box.val;
               ^^^ prefix
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=value kind=field replace=main.tspp#prefix suffix=": string" matches=0,1,2
```

### Complete a generic method inferred from a later use

A method signature uses the type argument a later call pins down.

```tspp main.tspp
struct Box<Value> {
    value: Value;
}

extension<Value> of Box<Value> {
    get(): Value {
        return this.value;
    }
}

declare function make<Value>(): Box<Value>;
declare function take(box: Box<int32>): void;

function read(): void {
    const box = make();
    box.ge;
        ^^ prefix
    take(box);
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=get kind=method replace=main.tspp#prefix suffix="(): int32" insert="get()" matches=0,1
```

### Complete a constrained parameter member

A type parameter exposes members declared by its constraint.

```tspp main.tspp
interface Named {
    name: string;
}

function nameOf<Value: Named>(value: Value): string {
    return value.na;
                 ^^ prefix
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=name kind=field replace=main.tspp#prefix suffix=": string" matches=0,1
```

### Complete a newtype backing member

A newtype exposes members selected through its backing value.

```tspp main.tspp
newtype User = { name: string };

function nameOf(user: User): string {
    return user.na;
                ^^ prefix
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=name kind=field replace=main.tspp#prefix suffix=": string" matches=0,1
```

### Complete primitive extension members

Primitive values expose their implicit extension methods.

```tspp main.tspp
function isEmpty(value: string): boolean {
    return value.isE;
                 ^^^ prefix
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=isEmpty kind=property replace=main.tspp#prefix suffix=": boolean" matches=0,1,2
@completion.item label=isWellFormed kind=method replace=main.tspp#prefix suffix="(): boolean" insert="isWellFormed()" matches=0,1,3
```

### Complete a common union member once

Union member completion includes only members available on every possible receiver.

```tspp main.tspp
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

```query completion main.tspp#common_prefix@end trigger=.
@completion.item label=label kind=field replace=main.tspp#common_prefix suffix=": string" matches=0,1
```

```query completion main.tspp#partial_prefix@end trigger=.
@completion.none
```

### Omit an unavailable union operation

A union only offers members that every possible value can use in the same way.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.none
```

### Complete intersection members

An intersection exposes members from each constituent.

```tspp main.tspp
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

```query completion main.tspp#prefix@end trigger=.
@completion.item label=id kind=field replace=main.tspp#prefix suffix=": int32" matches=0
@completion.item label=into kind=method replace=main.tspp#prefix suffix="(): U" description="as Into<U>" insert="into()" matches=0
@completion.item label=tryInto kind=method replace=main.tspp#prefix suffix="(): Result<U, U.Error>" description="as TryInto<U>" insert="tryInto()" matches=3
```

### Complete an associated constant

A nominal type receiver exposes its associated values.

```tspp main.tspp
struct Buffer {
    const Width: uint = 8;
}

const width = Buffer.Wi;
                     ^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=Width kind=associated_const replace=main.tspp#prefix suffix=": uint64" matches=0,1
```

### Complete an associated type

A constrained type parameter exposes its associated types.

```tspp main.tspp
interface Collection {
    type Item;
}

function item<T: Collection>(): T.Ite;
                                  ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=Item kind=associated_type replace=main.tspp#prefix suffix=": Collection.Item" matches=0,1,2
```

### Complete a static member through a type alias

A type alias exposes static members from its target declaration.

```tspp main.tspp
struct Buffer {
    const Width: uint = 8;
}

type BufferAlias = Buffer;
const width = BufferAlias.Wi;
                          ^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=Width kind=associated_const replace=main.tspp#prefix suffix=": uint64" matches=0,1
```

### Complete a namespace member

A namespace receiver exposes the exports of its target module.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import * as library from "./library.tspp";

library.gr;
        ^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=greet kind=function replace=main.tspp#prefix suffix="(): void" insert="greet()" matches=0,1
```

### Complete a namespace type

A namespace path exposes exported type declarations in type positions.

```tspp library.tspp
export struct Packet {}

export function PacketValue(): void {}
```

```tspp main.tspp
import * as library from "./library.tspp";

declare const packet: library.Pac;
                              ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=Packet kind=struct replace=main.tspp#prefix matches=0,1,2
```

### Complete a nested namespace type

Type paths include intermediate namespaces and filter their final declarations by use.

```tspp model.tspp
export struct Packet {}

export function PacketValue(): void {}
```

```tspp library.tspp
export * as models from "./model";
```

```tspp main.tspp
import * as library from "./library";

declare const packet: library.mod;
                              ^^^ name
```

```query completion main.tspp#name@end
@completion.item label=models kind=module replace=main.tspp#name matches=0,1,2
```

```tspp main.tspp type
import * as library from "./library";

declare const packet: library.models.Pac;
                                     ^^^ name
```

```query completion main.tspp#name@end
@completion.item label=Packet kind=struct replace=main.tspp#name matches=0,1,2
```

### Complete a namespace inside a resolved type path

Completion selects the namespace preceding the edited path segment.

```tspp model.tspp
export struct Packet {}
```

```tspp library.tspp
export * as models from "./model";
```

```tspp main.tspp
import * as library from "./library";

declare const packet: library.models.Packet;
                              ^^^ namespace
                              ^^^^^^ name
                                     ^^^ prefix
                                     ^^^^^^ type
```

```query completion main.tspp#namespace@end
@completion.item label=models kind=module replace=main.tspp#name matches=0,1,2
```

```query completion main.tspp#prefix@end
@completion.item label=Packet kind=struct replace=main.tspp#type matches=0,1,2
```

### Complete members from the current declaration

Member completion uses the receiver selected after each edit.

```tspp main.tspp
struct Box {
    value: string;
}

declare const box: Box;
const selected = box.val;
                     ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=value kind=field replace=main.tspp#prefix suffix=": string" matches=0,1,2
```

```tspp main.tspp change
struct Box {
    count: int32;
}

declare const box: Box;
const selected = box.cou;
                     ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=count kind=field replace=main.tspp#prefix suffix=": int32" matches=0,1,2
```

```diff main.tspp
@@ -1,3 +1,3 @@
 struct Box {
-    count: int32;
+    count: boolean;
 }
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=count kind=field replace=main.tspp#prefix suffix=": boolean" matches=0,1,2
```

### Complete blanket extensions from current conformance

Blanket extension completion follows the receiver's current interface conformance.

```tspp main.tspp
newtype interface Named {}

struct User implements Named {}

extension<Value: Named> of Value {
    displayName(): string {
        return "user";
    }
}

declare const user: User;
const result = user.dis;
                    ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=displayName kind=method replace=main.tspp#prefix suffix="(): string" insert="displayName()" matches=0,1,2
```

```diff main.tspp
@@ -3 +3 @@
-struct User implements Named {}
+struct User {}
```

```query completion main.tspp#prefix@end trigger=.
@completion.none
```

```diff main.tspp
@@ -3 +3 @@
-struct User {}
+struct User implements Named {}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=displayName kind=method replace=main.tspp#prefix suffix="(): string" insert="displayName()" matches=0,1,2
```

### Return no members for an unresolved receiver

An unresolved receiver has no member completion candidates.

```tspp main.tspp
struct Box {
    value: string;
}

declare const box: Box;
const selected = box.val;
                     ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=value kind=field replace=main.tspp#prefix suffix=": string" matches=0,1,2
```

```diff main.tspp
@@ -5,3 +5,3 @@
 declare const box: Box;
-const selected = box.val;
+const selected = missing.val;
-                     ^^^ prefix
+                         ^^^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.none
```

### Complete a member throughout module typing

Member completion remains available throughout incomplete edits and returns the final member.

```tspp main.tspp
// module
```

```tspp main.tspp type
// module
^^^^^^^^^ module:start

class World {
^ world_range:start
      ^^^^^ world_selection
    name: string;
    ^^^^^^^^^^^^ world_name_range
    ^^^^ world_name_selection
}
^ world_range:end

struct Position {
^ position_range:start
       ^^^^^^^^ position_selection
    x: float64;
    ^^^^^^^^^^ position_x_range
    ^ position_x_selection
    y: float64;
    ^^^^^^^^^^ position_y_range
    ^ position_y_selection
}
^ position_range:end

class Player {
^ player_range:start
      ^^^^^^ player_selection
    world: World;
    ^^^^^^^^^^^^ player_world_range
    ^^^^^ player_world_selection
           ^^^^^ world_reference
    position: Position;
    ^^^^^^^^^^^^^^^^^^ player_position_range
    ^^^^^^^^ player_position_selection
              ^^^^^^^^ position_reference
}
^ player_range:end

const playerCount = 1;
^^^^^^^^^^^^^^^^^^^^^ player_count_range
      ^^^^^^^^^^^ player_count_selection

declare const player: Player;
                      ^^^^^^ player_type
const selected = player.wo;
      ^^^^^^^^ selected
                 ^^^^^^ player_reference
                        ^^ prefix
^^^^^^^^^^^^^^^^^^^^^^^^^^ module:end
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=world kind=field replace=main.tspp#prefix suffix=": World" matches=0,1
```

```query goto_definition main.tspp#world_reference
@goto_definition.target origin=main.tspp#world_reference location=main.tspp#world_range selection=main.tspp#world_selection symbol=main.tspp#World@1
```

```query hover main.tspp#player_type
@hover.item index=0 declaration="class Player" location=main.tspp#player_range selection=main.tspp#player_selection range=main.tspp#player_type
```

```query outline main.tspp
@outline.symbol depth=0 name=World kind=class range=main.tspp#world_range selection=main.tspp#world_selection
@outline.symbol depth=1 name=name kind=field detail=string range=main.tspp#world_name_range selection=main.tspp#world_name_selection
@outline.symbol depth=0 name=Position kind=struct range=main.tspp#position_range selection=main.tspp#position_selection
@outline.symbol depth=1 name=x kind=field detail=float64 range=main.tspp#position_x_range selection=main.tspp#position_x_selection
@outline.symbol depth=1 name=y kind=field detail=float64 range=main.tspp#position_y_range selection=main.tspp#position_y_selection
@outline.symbol depth=0 name=Player kind=class range=main.tspp#player_range selection=main.tspp#player_selection
@outline.symbol depth=1 name=world kind=field detail=World range=main.tspp#player_world_range selection=main.tspp#player_world_selection
@outline.symbol depth=1 name=position kind=field detail=Position range=main.tspp#player_position_range selection=main.tspp#player_position_selection
@outline.symbol depth=0 name=playerCount kind=constant detail=1 range=main.tspp#player_count_range selection=main.tspp#player_count_selection
@outline.symbol depth=0 name=player kind=constant detail=Player range=main.tspp:19:1-19:29 selection=main.tspp:19:15-19:21
@outline.symbol depth=0 name=selected kind=constant detail="<error>" range=main.tspp:20:1-20:27 selection=main.tspp#selected
```

```query inlay_hints main.tspp#module
@inlay_hints.hint position=main.tspp#player_count_selection@end label=": 1" kind=type
@inlay_hints.hint position=main.tspp#selected@end label=": <error>" kind=type
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#world_selection type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#world_name_selection type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#position_selection type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#position_x_selection type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#position_y_selection type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#player_selection type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#player_world_selection type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#world_reference type=class
@semantic_tokens.token range=main.tspp#player_position_selection type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#position_reference type=struct
@semantic_tokens.token range=main.tspp#player_count_selection type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp:19:15-19:21 type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#player_type type=class
@semantic_tokens.token range=main.tspp#selected type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#player_reference type=variable modifiers=readonly
```

## Types

### Complete a nominal type

Type positions include visible type declarations.

```tspp main.tspp
struct FixturePoint {
    x: int32;
}

declare const point: FixturePoin;
                     ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=FixturePoint kind=struct replace=main.tspp#prefix matches=0,1,2,3,4,5,6,7,8,9,10
```

### Exclude value-only declarations from a type position

Type completion follows the language namespaces rather than returning lexical name matches.

```tspp main.tspp
const PacketValue = 1;

declare const packet: PacketV;
                      ^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.none
```

### Complete a generic type parameter

A generic parameter remains visible throughout its declaration.

```tspp main.tspp
function identity<Value>(value: Value): Value {
                                        ^^^^^ prefix
    return value;
}
```

```query completion main.tspp#prefix@end
@completion.item label=Value kind=type_parameter replace=main.tspp#prefix matches=0,1,2,3,4
@completion.item label=ValueEquality kind=struct replace=main.tspp#prefix suffix="<T>" matches=0,1,2,3,4
```

### Complete a type with an explicit lifetime

Type details preserve a general lifetime parameter in borrowed forms.

```tspp main.tspp
type BorrowedFields<T, const L: Lifetime> = T;

type Alias<const L: Lifetime> = BorrowedFields<unknown, L>;
                                ^^^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=BorrowedFields kind=type_alias replace=main.tspp#prefix suffix="<T, const L: Lifetime>" matches=0,1,2,3,4,5,6,7,8,9,10,11,12,13
```

### Complete an imported type through a re-export

Type completion preserves the declaration kind through module aliases.

```tspp model.tspp
export struct Packet {}
```

```tspp library.tspp
export { Packet } from "./model.tspp";
```

```tspp main.tspp
import { Packet } from "./library.tspp";

declare const packet: Pack;
                      ^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=Packet kind=struct replace=main.tspp#prefix matches=0,1,2,3
```

## Enum Members

### Complete an enum member

Member completion uses the enum receiver type.

```tspp main.tspp
enum Color {
    Red,
    Blue,
}

const color = Color.R;
                    ^ prefix
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=Red kind=enum_member replace=main.tspp#prefix suffix=": Color.Red" matches=0
```

### Complete a discriminated union discriminator

Discriminated unions expose their shared discriminator with every possible tag.

```tspp main.tspp
newtype Status =
    | { type: "ok"; value: string }
    | { type: "error"; error: int32 };

function statusType(status: Status): string {
    return status.type;
                  ^^^^ prefix
}
```

```query completion main.tspp#prefix@end trigger=.
@completion.item label=type kind=field replace=main.tspp#prefix suffix=": \"ok\" | \"error\"" matches=0,1,2,3
```

## Constructors

### Preserve authored call arguments

Completing a function, method, or constructor name preserves its existing arguments and fields.

```tspp main.tspp
function fixtureGreet(name: string): string {
    return name;
}

function fixtureEcho<Value>(value: Value): Value {
    return value;
}

class FixturePlayer {
    constructor(name: string) {}

    fixtureSpeak(name: string): string {
        return name;
    }
}

newtype FixtureId = string;

struct FixturePoint {
    x: int32;
}

const greeting = fixtureGreet /* greeting */ ("Ada");
                 ^^^^^^^^^^^^ function

const echoed = fixtureEcho<string>("Ada");
               ^^^^^^^^^^^ generic

const player = new FixturePlayer("Ada");
                   ^^^^^^^^^^^^^ constructor

const message = player.fixtureSpeak("Ada");
                       ^^^^^^^^^^^^ method

const id = FixtureId("Ada");
           ^^^^^^^^^ newtype

const point = FixturePoint { x: 1 };
              ^^^^^^^^^^^^ structure
```

```query completion main.tspp#function@end
@completion.item label=fixtureGreet kind=function replace=main.tspp#function suffix="(name: string): string" matches=0,1,2,3,4,5,6,7,8,9,10,11
```

```query completion main.tspp#generic@end
@completion.item label=fixtureEcho kind=function replace=main.tspp#generic suffix="(value: Value): Value" matches=0,1,2,3,4,5,6,7,8,9,10
```

```query completion main.tspp#constructor@end
@completion.item label=FixturePlayer kind=class replace=main.tspp#constructor suffix="(name: string): FixturePlayer" matches=0,1,2,3,4,5,6,7,8,9,10,11,12
```

```query completion main.tspp#method@end
@completion.item label=fixtureSpeak kind=method replace=main.tspp#method suffix="(name: string): string" matches=0,1,2,3,4,5,6,7,8,9,10,11
```

```query completion main.tspp#newtype@end
@completion.item label=FixtureId kind=constructor replace=main.tspp#newtype suffix="(string): FixtureId" matches=0,1,2,3,4,5,6,7,8
```

```query completion main.tspp#structure@end
@completion.item label=FixturePoint kind=struct replace=main.tspp#structure matches=0,1,2,3,4,5,6,7,8,9,10,11
```

### Complete a constructable class

New expressions include constructable nominal values.

```tspp main.tspp
class Widget {
    constructor(name: string) {}
}

const widget = new Widget;
                   ^^^ prefix
                   ^^^^^^ token
```

```query completion main.tspp#prefix@end
@completion.item label=Widget kind=class replace=main.tspp#token suffix="(name: string): Widget" insert="Widget(${1:name})$0" snippet=true matches=0,1,2
```

### Complete a class through its default constructor

A class declaring no constructor completes as its default `new` form.

```tspp main.tspp
class Gadget {}

const gadget = new Gadge;
                   ^^^^^ prefix
                   ^^^^^^ token
```

```query completion main.tspp#prefix@end
@completion.item label=Gadget kind=class replace=main.tspp#prefix suffix="(): Gadget" insert="Gadget()" matches=0,1,2,3,4
```

### Complete a derived class through its base constructor

A derived class declaring no constructor completes with its base class's parameters.

```tspp main.tspp
class Base {
    constructor(name: string) {}
}

class Derived extends Base {}

const derived = new Derive;
                    ^^^^^^ prefix
                    ^^^^^^^ token
```

```query completion main.tspp#prefix@end
@completion.item label=Derived kind=class replace=main.tspp#prefix suffix="(name: string): Derived" insert="Derived(${1:name})$0" snippet=true matches=0,1,2,3,4,5
```

### Complete a struct expression

A struct value completion can insert every required field.

```tspp main.tspp
struct FixturePoint {
    x: int32;
    y: int32;
}

const result = FixturePoin;
               ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=FixturePoint kind=struct replace=main.tspp#prefix insert="FixturePoint { x: ${1}, y: ${2} }$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10
```

### Complete dollar-prefixed fields

Struct and object completions preserve dollar signs in their field names.

```tspp main.tspp
struct Fixture$Point {
    $value: int32;
}

const result = Fixture$Poin;
               ^^^^^^^^^^^^ structure
const point: Fixture$Point = {
    $val
    ^^^^ field
};
```

```query completion main.tspp#structure@end
@completion.item label="Fixture$Point" kind=struct replace=main.tspp#structure insert="Fixture\\$Point { \\$value: ${1} }$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10,11
```

```query completion main.tspp#field@end
@completion.item label="$value" kind=field replace=main.tspp#field suffix=": int32" insert="\\$value: ${1}" snippet=true matches=0,1,2,3
```

### Complete quoted and numeric fields

Struct and object completions format quoted and numeric property keys.

```tspp main.tspp
struct FixtureRecord {
    "display-name": string;
    42: int32;
}

const result = FixtureRecor;
               ^^^^^^^^^^^^ structure
const record: FixtureRecord = { };
                               ^ field
```

```query completion main.tspp#structure@end
@completion.item label=FixtureRecord kind=struct replace=main.tspp#structure insert="FixtureRecord { \"display-name\": ${1}, 42: ${2} }$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10,11
```

```query completion main.tspp#field@start
@completion.item label=display-name kind=field replace=main.tspp#field suffix=": string" insert="\"display-name\": ${1}" snippet=true
@completion.item label=42 kind=field replace=main.tspp#field suffix=": int32" insert="42: ${1}" snippet=true
```

### Complete a newtype constructor

A newtype completion includes its constructor signature and call snippet.

```tspp main.tspp
newtype UserId = string;

const result = UserI;
               ^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=UserId kind=constructor replace=main.tspp#prefix suffix="(string): UserId" insert="UserId(${1})$0" snippet=true matches=0,1,2,3,4
```

### Complete every newtype constructor once

A selected newtype expands into its constructor overloads.

```tspp main.tspp
newtype Choice = string | int32;

const result = Choice(1);
               ^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=Choice kind=constructor replace=main.tspp#prefix suffix="(string): Choice" matches=0,1,2,3,4,5
@completion.item label=Choice kind=constructor replace=main.tspp#prefix suffix="(int32): Choice" matches=0,1,2,3,4,5
@completion.item label=Choice kind=constructor replace=main.tspp#prefix suffix="(string | int32): Choice" matches=0,1,2,3,4,5
```

### Complete a newtype constructor from its current backing type

Newtype completion updates when its backing type changes.

```tspp main.tspp
newtype UserId = string;

const result = UserI;
               ^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=UserId kind=constructor replace=main.tspp#prefix suffix="(string): UserId" insert="UserId(${1})$0" snippet=true matches=0,1,2,3,4
```

```diff main.tspp
@@ -1 +1 @@
-newtype UserId = string;
+newtype UserId = int32;
```

```query completion main.tspp#prefix@end
@completion.item label=UserId kind=constructor replace=main.tspp#prefix suffix="(int32): UserId" insert="UserId(${1})$0" snippet=true matches=0,1,2,3,4
```

### Complete an imported newtype constructor reference

An imported newtype can be completed before its argument list is written.

```tspp library.tspp
export newtype FixtureId = string;
```

```tspp main.tspp
import { FixtureId } from "./library";
```

```tspp main.tspp type
import { FixtureId } from "./library";

const id = FixtureId;
           ^^^^^^^^^ name
```

```query completion main.tspp#name@end
@completion.item label=FixtureId kind=constructor replace=main.tspp#name suffix="(string): FixtureId" insert="FixtureId(${1})$0" snippet=true matches=0,1,2,3,4,5,6,7,8
```

## Object Literals

### Complete a missing field

Object completion omits fields already present in the literal.

```tspp main.tspp
struct Rectangle {
    width: int32;
    height: int32;
}

const rectangle: Rectangle = {
    width: 10,
    heigh
    ^^^^^ prefix
};
```

```query completion main.tspp#prefix@end
@completion.item label=height kind=field replace=main.tspp#prefix suffix=": int32" insert="height: ${1}" snippet=true matches=0,1,2,3,4
```

### Substitute a generic field type

A generic object field uses the applied type argument.

```tspp main.tspp
struct Box<Value> {
    value: Value;
}

const box: Box<string> = {
    val
    ^^^ prefix
};
```

```query completion main.tspp#prefix@end
@completion.item label=value kind=field replace=main.tspp#prefix suffix=": string" insert="value: ${1}" snippet=true matches=0,1,2
```

### Complete a generic field inferred from a later use

An object field uses the type argument a later call pins down.

```tspp main.tspp
struct Box<Value> {
    value: Value;
}

declare function wrap<Value>(box: Box<Value>): Box<Value>;
declare function take(box: Box<int32>): void;

function main(): void {
    const box = wrap({
        val
        ^^^ prefix
    });
    take(box);
}
```

```query completion main.tspp#prefix@end
@completion.item label=value kind=field replace=main.tspp#prefix suffix=": int32" insert="value: ${1}" snippet=true matches=0,1,2
```

### Complete a nested field

Nested object literals use the expected type at their own position.

```tspp main.tspp
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

```query completion main.tspp#prefix@end
@completion.item label=street kind=field replace=main.tspp#prefix suffix=": string" insert="street: ${1}" snippet=true matches=0,1,2
```

### Use object shorthand for a visible field value

When a matching binding is visible, field completion inserts the shorthand form.

```tspp main.tspp
struct Rectangle {
    width: int32;
    height: int32;
}

const height: int32 = 20;
const rectangle: Rectangle = {
    width: 10,
    heigh
    ^^^^^ prefix
};
```

```query completion main.tspp#prefix@end
@completion.item label=height kind=field replace=main.tspp#prefix suffix=": int32" matches=0,1,2,3,4
```

### Complete a visible shorthand without a contextual type

An object literal can use any visible value as a shorthand property.

```tspp main.tspp
const height: int32 = 20;
const rectangle = {
    heigh
    ^^^^^ prefix
};
```

```query completion main.tspp#prefix@end
@completion.item label=height kind=field replace=main.tspp#prefix suffix=": int32" matches=0,1,2,3,4
```

### Distinguish type declarations from shorthand values

Object shorthand completion suggests value bindings with the requested prefix.

```tspp main.tspp
struct FixturePosition {
    x: int32;
}

const FixturePoint = 1;

const object = {
    FixtureP
    ^^^^^^^^ prefix
};
```

```query completion main.tspp#prefix@end
@completion.item label=FixturePoint kind=field replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4,5,6,7
```

### Omit a field supplied by a spread

A spread supplies its fields to the surrounding object.

```tspp main.tspp
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

```query completion main.tspp#prefix@end
@completion.none
```

### Omit a field supplied by a nominal spread

A nominal spread supplies its selected instance fields.

```tspp main.tspp
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

```query completion main.tspp#prefix@end
@completion.none
```

## Call Arguments

### Prefer the expected argument type

Candidates matching the active parameter type rank ahead of lexical peers.

```tspp main.tspp
function consume(value: int32): void {}

const candidateAlpha: string = "";
const candidateZulu: int32 = 1;

consume(candidate);
        ^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=candidateZulu kind=constant replace=main.tspp#prefix suffix=": int32" preselect=true matches=0,1,2,3,4,5,6,7,8
@completion.item label=candidateAlpha kind=constant replace=main.tspp#prefix suffix=": string" matches=0,1,2,3,4,5,6,7,8
```

### Prefer the expected constructor argument type

Class and newtype arguments rank candidates by their recorded parameter types.

```tspp main.tspp
class NumberBox {
    constructor(value: int32) {}
}
newtype NumberId = int32;

const candidateAlpha: string = "";
const candidateZulu: int32 = 1;

new NumberBox(candidate);
              ^^^^^^^^^ classArgument
NumberId(candidate);
         ^^^^^^^^^ newtypeArgument
```

```query completion main.tspp#classArgument@end
@completion.item label=candidateZulu kind=constant replace=main.tspp#classArgument suffix=": int32" preselect=true matches=0,1,2,3,4,5,6,7,8
@completion.item label=candidateAlpha kind=constant replace=main.tspp#classArgument suffix=": string" matches=0,1,2,3,4,5,6,7,8
```

```query completion main.tspp#newtypeArgument@end
@completion.item label=candidateZulu kind=constant replace=main.tspp#newtypeArgument suffix=": int32" preselect=true matches=0,1,2,3,4,5,6,7,8
@completion.item label=candidateAlpha kind=constant replace=main.tspp#newtypeArgument suffix=": string" matches=0,1,2,3,4,5,6,7,8
```

### Prefer an exact name over the expected type

Lexical quality precedes type relevance.

```tspp main.tspp
function consume(value: int32): void {}

const candidate: string = "";
const candidateValue: int32 = 1;

consume(candidate);
        ^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=candidate kind=constant replace=main.tspp#prefix suffix=": string" matches=0,1,2,3,4,5,6,7,8
@completion.item label=candidateValue kind=constant replace=main.tspp#prefix suffix=": int32" matches=0,1,2,3,4,5,6,7,8
```

### Exclude callee parameters

Parameter declarations do not enter the caller's lexical scope.

```tspp main.tspp
function consume(target: int32): void {}

const tangible: int32 = 1;

consume(tangible);
        ^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=tangible kind=constant replace=main.tspp#prefix suffix=": int32" preselect=true matches=0,1,2,3,4,5,6,7
```

## Imports

### Complete a named import

Import clause completion reads the target module exports.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { gre } from "./library.tspp";
         ^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=greet kind=function replace=main.tspp#prefix suffix="(): void" matches=0,1,2
```

### Exclude an already imported name

Named import completion omits exports already present in the same clause.

```tspp library.tspp
export function greet(): void {}
export function grow(): void {}
```

```tspp main.tspp
import { greet, gr } from "./library.tspp";
                ^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=grow kind=function replace=main.tspp#prefix suffix="(): void" matches=0,1
```

### Complete a type declaration in an import

A named import exposes declarations from their original symbol spaces.

```tspp library.tspp
export type Options = {
    enabled: boolean,
};

export function open(): void {}
```

```tspp main.tspp
import { Opt } from "./library.tspp";
         ^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=Options kind=type_alias replace=main.tspp#prefix matches=0,1,2
```

### Complete an export beside an import alias

An import alias does not hide a different export with the same name.

```tspp library.tspp
export function greet(): void {}

export function grow(): void {}
```

```tspp main.tspp
import { greet as grow, gr } from "./library";
                        ^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=grow kind=function replace=main.tspp#prefix suffix="(): void" matches=0,1
```

### Replace an imported name before its alias

Completion replaces the exported name and preserves the local alias.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { greetWrong as local } from "./library";
         ^^^ prefix
         ^^^^^^^^^^ name
```

```query completion main.tspp#prefix@end
@completion.item label=greet kind=function replace=main.tspp#name suffix="(): void" matches=0,1,2
```

### Omit exports from an import alias

An alias declares a local name and does not complete target module exports.

```tspp library.tspp
export function greet(): void {}

export function grow(): void {}
```

```tspp main.tspp
import { greet as gr } from "./library";
                  ^^ alias
```

```query completion main.tspp#alias@end
@completion.none
```

### Complete a re-exported name

Import completion exposes the name exported by the target module.

```tspp model.tspp
export function createPacket(): void {}
```

```tspp library.tspp
export { createPacket as packet } from "./model.tspp";
```

```tspp main.tspp
import { pack } from "./library.tspp";
         ^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=packet kind=function replace=main.tspp#prefix suffix="(): void" matches=0,1,2,3
```

## Auto Imports

### Auto import a type beside a value name match

A value-only export does not hide a matching type declaration.

```tspp library.tspp
export const ZbrValue = 1;

export struct Zebra {}
```

```tspp main.tspp

^ insertion
declare const value: Zbr;
                     ^^^ name
```

```query completion main.tspp#name@end include_auto_imports=true
@completion.item label=Zebra kind=struct replace=main.tspp#name description="from ./library" auto_import=true matches=0,2,3
@completion.additional_edit item=0 range=main.tspp#insertion text="import { Zebra } from \"./library\";\n"
```

### Complete constructors before and after importing them

Auto imports, named imports, and namespace imports use the same constructor insertions.

```tspp library.tspp
export struct FixturePoint {
    x: int32;
}

export class FixturePlayer {
    constructor(name: string) {}
}

export newtype FixtureKey = string;
```

```tspp main.tspp

^ insertion
const point = FixturePoin;
              ^^^^^^^^^^^ point

const player = new FixturePlay;
                   ^^^^^^^^^^^ player

const id = FixtureKe;
           ^^^^^^^^^ id
```

```query completion main.tspp#point@end include_auto_imports=true
@completion.item label=FixturePoint kind=struct replace=main.tspp#point description="from ./library" insert="FixturePoint { x: ${1} }$0" snippet=true auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=main.tspp#insertion text="import { FixturePoint } from \"./library\";\n"
```

```query completion main.tspp#player@end include_auto_imports=true
@completion.item label=FixturePlayer kind=class replace=main.tspp#player suffix="(name: string): FixturePlayer" description="from ./library" insert="FixturePlayer(${1:name})$0" snippet=true auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=main.tspp#insertion text="import { FixturePlayer } from \"./library\";\n"
```

```query completion main.tspp#id@end include_auto_imports=true
@completion.item label=FixtureKey kind=constructor replace=main.tspp#id suffix="(string): FixtureKey" description="from ./library" insert="FixtureKey(${1})$0" snippet=true auto_import=true matches=0,1,2,3,4,5,6,7,8
@completion.additional_edit item=0 range=main.tspp#insertion text="import { FixtureKey } from \"./library\";\n"
```

```tspp main.tspp change
import { FixturePoint, FixturePlayer, FixtureKey } from "./library";

const point = FixturePoin;
              ^^^^^^^^^^^ point

const player = new FixturePlay;
                   ^^^^^^^^^^^ player

const id = FixtureKe;
           ^^^^^^^^^ id
```

```query completion main.tspp#point@end include_auto_imports=true
@completion.item label=FixturePoint kind=struct replace=main.tspp#point insert="FixturePoint { x: ${1} }$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10
```

```query completion main.tspp#player@end include_auto_imports=true
@completion.item label=FixturePlayer kind=class replace=main.tspp#player suffix="(name: string): FixturePlayer" insert="FixturePlayer(${1:name})$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10
```

```query completion main.tspp#id@end include_auto_imports=true
@completion.item label=FixtureKey kind=constructor replace=main.tspp#id suffix="(string): FixtureKey" insert="FixtureKey(${1})$0" snippet=true matches=0,1,2,3,4,5,6,7,8
```

```tspp main.tspp change
import * as library from "./library";

const point = library.FixturePoin;
                      ^^^^^^^^^^^ point

const player = new library.FixturePlay;
                           ^^^^^^^^^^^ player

const id = library.FixtureKe;
                   ^^^^^^^^^ id
```

```query completion main.tspp#point@end
@completion.item label=FixturePoint kind=struct replace=main.tspp#point insert="FixturePoint { x: ${1} }$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10
```

```query completion main.tspp#player@end
@completion.item label=FixturePlayer kind=class replace=main.tspp#player suffix="(name: string): FixturePlayer" insert="FixturePlayer(${1:name})$0" snippet=true matches=0,1,2,3,4,5,6,7,8,9,10
```

```query completion main.tspp#id@end
@completion.item label=FixtureKey kind=constructor replace=main.tspp#id suffix="(string): FixtureKey" insert="FixtureKey(${1})$0" snippet=true matches=0,1,2,3,4,5,6,7,8
```

### Complete an exported function with an import edit

Auto import completion returns the symbol and import patch together.

```tspp library.tspp
export function greetFixture(): void {}
```

```tspp main.tspp

^ insertion
function main(): void {
    greetFix;
    ^^^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=greetFixture kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="greetFixture()" auto_import=true matches=0,1,2,3,4,5,6,7
@completion.additional_edit item=0 range=main.tspp#insertion text="import { greetFixture } from \"./library\";\n"
```

### Auto import a star re-exported symbol

A name visible only through `export *` still completes with its import patch.

```tspp core.tspp
export function starredGreeting(): void {}
```

```tspp library.tspp
export * from "./core";
```

```tspp main.tspp

^ insertion
function main(): void {
    starredGree;
    ^^^^^^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=starredGreeting kind=function replace=main.tspp#prefix suffix="(): void" description="from ./core" insert="starredGreeting()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=main.tspp#insertion text="import { starredGreeting } from \"./core\";\n"
@completion.item label=starredGreeting kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="starredGreeting()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=1 range=main.tspp#insertion text="import { starredGreeting } from \"./library\";\n"
```

### Shadow a star re-export with a nearer named export

A re-exporting module's own declaration hides the starred name behind it.

```tspp core.tspp
export function shadowedGreeting(): void {}
```

```tspp library.tspp
export * from "./core";
export function shadowedGreeting(): void {}
```

```tspp main.tspp

^ insertion
function main(): void {
    shadowedGree;
    ^^^^^^^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=shadowedGreeting kind=function replace=main.tspp#prefix suffix="(): void" description="from ./core" insert="shadowedGreeting()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10,11
@completion.additional_edit item=0 range=main.tspp#insertion text="import { shadowedGreeting } from \"./core\";\n"
@completion.item label=shadowedGreeting kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="shadowedGreeting()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10,11
@completion.additional_edit item=1 range=main.tspp#insertion text="import { shadowedGreeting } from \"./library\";\n"
```

### Refresh completion with a longer prefix

A retriggered request filters candidates with the latest prefix.

```tspp library.tspp
export function refreshTarget(): void {}
```

```tspp main.tspp

^ insertion
function main(): void {
    refreshTarge;
    ^^^^^^^^^^^^ prefix
}
```

```query completion main.tspp#prefix@end trigger=incomplete include_auto_imports=true
@completion.item label=refreshTarget kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="refreshTarget()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10,11
@completion.additional_edit item=0 range=main.tspp#insertion text="import { refreshTarget } from \"./library\";\n"
```

### Report a truncated completion list

A truncated short-prefix list reports that more matching candidates are available.

```tspp library.tspp
export type $x00 = int32;
export type $x01 = int32;
export type $x02 = int32;
export type $x03 = int32;
export type $x04 = int32;
export type $x05 = int32;
export type $x06 = int32;
export type $x07 = int32;
export type $x08 = int32;
export type $x09 = int32;
export type $x10 = int32;
export type $x11 = int32;
export type $x12 = int32;
export type $x13 = int32;
export type $x14 = int32;
export type $x15 = int32;
export type $x16 = int32;
export type $x17 = int32;
export type $x18 = int32;
export type $x19 = int32;
export type $x20 = int32;
export type $x21 = int32;
export type $x22 = int32;
export type $x23 = int32;
export type $x24 = int32;
export type $x25 = int32;
export type $x26 = int32;
export type $x27 = int32;
export type $x28 = int32;
export type $x29 = int32;
export type $x30 = int32;
export type $x31 = int32;
export type $x32 = int32;
export type $x33 = int32;
export type $x34 = int32;
export type $x35 = int32;
export type $x36 = int32;
export type $x37 = int32;
export type $x38 = int32;
export type $x39 = int32;
export type $x40 = int32;
export type $x41 = int32;
export type $x42 = int32;
export type $x43 = int32;
export type $x44 = int32;
export type $x45 = int32;
export type $x46 = int32;
export type $x47 = int32;
export type $x48 = int32;
export type $x49 = int32;
export type $x50 = int32;
```

```tspp main.tspp

^ insertion
type Selected = $;
                ^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.list incomplete=true
@completion.item label="$x00" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=0 range=main.tspp#insertion text="import { $x00 } from \"./library\";\n"
@completion.item label="$x01" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=1 range=main.tspp#insertion text="import { $x01 } from \"./library\";\n"
@completion.item label="$x02" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=2 range=main.tspp#insertion text="import { $x02 } from \"./library\";\n"
@completion.item label="$x03" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=3 range=main.tspp#insertion text="import { $x03 } from \"./library\";\n"
@completion.item label="$x04" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=4 range=main.tspp#insertion text="import { $x04 } from \"./library\";\n"
@completion.item label="$x05" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=5 range=main.tspp#insertion text="import { $x05 } from \"./library\";\n"
@completion.item label="$x06" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=6 range=main.tspp#insertion text="import { $x06 } from \"./library\";\n"
@completion.item label="$x07" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=7 range=main.tspp#insertion text="import { $x07 } from \"./library\";\n"
@completion.item label="$x08" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=8 range=main.tspp#insertion text="import { $x08 } from \"./library\";\n"
@completion.item label="$x09" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=9 range=main.tspp#insertion text="import { $x09 } from \"./library\";\n"
@completion.item label="$x10" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=10 range=main.tspp#insertion text="import { $x10 } from \"./library\";\n"
@completion.item label="$x11" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=11 range=main.tspp#insertion text="import { $x11 } from \"./library\";\n"
@completion.item label="$x12" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=12 range=main.tspp#insertion text="import { $x12 } from \"./library\";\n"
@completion.item label="$x13" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=13 range=main.tspp#insertion text="import { $x13 } from \"./library\";\n"
@completion.item label="$x14" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=14 range=main.tspp#insertion text="import { $x14 } from \"./library\";\n"
@completion.item label="$x15" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=15 range=main.tspp#insertion text="import { $x15 } from \"./library\";\n"
@completion.item label="$x16" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=16 range=main.tspp#insertion text="import { $x16 } from \"./library\";\n"
@completion.item label="$x17" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=17 range=main.tspp#insertion text="import { $x17 } from \"./library\";\n"
@completion.item label="$x18" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=18 range=main.tspp#insertion text="import { $x18 } from \"./library\";\n"
@completion.item label="$x19" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=19 range=main.tspp#insertion text="import { $x19 } from \"./library\";\n"
@completion.item label="$x20" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=20 range=main.tspp#insertion text="import { $x20 } from \"./library\";\n"
@completion.item label="$x21" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=21 range=main.tspp#insertion text="import { $x21 } from \"./library\";\n"
@completion.item label="$x22" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=22 range=main.tspp#insertion text="import { $x22 } from \"./library\";\n"
@completion.item label="$x23" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=23 range=main.tspp#insertion text="import { $x23 } from \"./library\";\n"
@completion.item label="$x24" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=24 range=main.tspp#insertion text="import { $x24 } from \"./library\";\n"
@completion.item label="$x25" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=25 range=main.tspp#insertion text="import { $x25 } from \"./library\";\n"
@completion.item label="$x26" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=26 range=main.tspp#insertion text="import { $x26 } from \"./library\";\n"
@completion.item label="$x27" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=27 range=main.tspp#insertion text="import { $x27 } from \"./library\";\n"
@completion.item label="$x28" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=28 range=main.tspp#insertion text="import { $x28 } from \"./library\";\n"
@completion.item label="$x29" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=29 range=main.tspp#insertion text="import { $x29 } from \"./library\";\n"
@completion.item label="$x30" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=30 range=main.tspp#insertion text="import { $x30 } from \"./library\";\n"
@completion.item label="$x31" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=31 range=main.tspp#insertion text="import { $x31 } from \"./library\";\n"
@completion.item label="$x32" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=32 range=main.tspp#insertion text="import { $x32 } from \"./library\";\n"
@completion.item label="$x33" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=33 range=main.tspp#insertion text="import { $x33 } from \"./library\";\n"
@completion.item label="$x34" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=34 range=main.tspp#insertion text="import { $x34 } from \"./library\";\n"
@completion.item label="$x35" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=35 range=main.tspp#insertion text="import { $x35 } from \"./library\";\n"
@completion.item label="$x36" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=36 range=main.tspp#insertion text="import { $x36 } from \"./library\";\n"
@completion.item label="$x37" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=37 range=main.tspp#insertion text="import { $x37 } from \"./library\";\n"
@completion.item label="$x38" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=38 range=main.tspp#insertion text="import { $x38 } from \"./library\";\n"
@completion.item label="$x39" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=39 range=main.tspp#insertion text="import { $x39 } from \"./library\";\n"
@completion.item label="$x40" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=40 range=main.tspp#insertion text="import { $x40 } from \"./library\";\n"
@completion.item label="$x41" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=41 range=main.tspp#insertion text="import { $x41 } from \"./library\";\n"
@completion.item label="$x42" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=42 range=main.tspp#insertion text="import { $x42 } from \"./library\";\n"
@completion.item label="$x43" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=43 range=main.tspp#insertion text="import { $x43 } from \"./library\";\n"
@completion.item label="$x44" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=44 range=main.tspp#insertion text="import { $x44 } from \"./library\";\n"
@completion.item label="$x45" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=45 range=main.tspp#insertion text="import { $x45 } from \"./library\";\n"
@completion.item label="$x46" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=46 range=main.tspp#insertion text="import { $x46 } from \"./library\";\n"
@completion.item label="$x47" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=47 range=main.tspp#insertion text="import { $x47 } from \"./library\";\n"
@completion.item label="$x48" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=48 range=main.tspp#insertion text="import { $x48 } from \"./library\";\n"
@completion.item label="$x49" kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0
@completion.additional_edit item=49 range=main.tspp#insertion text="import { $x49 } from \"./library\";\n"
```

### Preserve an extension for an ambiguous relative path

An explicit extension keeps the target module unambiguous.

```tspp library.tspp
export function extensionGreeting(): void {}
```

```tspp library.d.tspp
export type LibraryDeclaration = string;
```

```tspp main.tspp

^ insertion
extensionGree
^^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=extensionGreeting kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library.tspp" insert="extensionGreeting()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10,11,12
@completion.additional_edit item=0 range=main.tspp#insertion text="import { extensionGreeting } from \"./library.tspp\";\n"
```

### Auto-import a type declaration

A type completion inserts a named import that preserves the declaration's symbol space.

```tspp library.tspp
export type Widget = {
    value: string,
};
```

```tspp main.tspp

^ insertion
type Alias = Widget;
             ^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=Widget kind=type_alias replace=main.tspp#prefix description="from ./library" auto_import=true matches=0,1,2,3,4,5
@completion.additional_edit item=0 range=main.tspp#insertion text="import { Widget } from \"./library\";\n"
```

### Auto-import a public builtin type

Builtin modules follow their public package exports.

```tspp main.tspp

^ insertion
declare const variable: ContextV;
                        ^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=ContextVar kind=class replace=main.tspp#prefix suffix="<T: Copy>" description="from destack:" auto_import=true matches=0,1,2,3,4,5,6,7
@completion.additional_edit item=0 range=main.tspp#insertion text="import { ContextVar } from \"destack:\";\n"
@completion.item label=ContextVar kind=class replace=main.tspp#prefix suffix="<T: Copy>" description="from tspp:context" auto_import=true matches=0,1,2,3,4,5,6,7
@completion.additional_edit item=1 range=main.tspp#insertion text="import { ContextVar } from \"tspp:context\";\n"
```

### Auto-import a default declaration

A default export produces a default import rather than a named import.

```tspp library.tspp
export default function defaultGreeting(name: string): string {
    return name;
}
```

```tspp main.tspp

^ insertion
const message = defaultGree;
                ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=defaultGreeting kind=function replace=main.tspp#prefix suffix="(name: string): string" description="from ./library" insert="defaultGreeting(${1:name})$0" snippet=true auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=main.tspp#insertion text="import defaultGreeting from \"./library\";\n"
```

### Omit an already imported default declaration

A default export already imported under a local name cannot add a second binding.

```tspp library.tspp
export default function fixtureDefaultConstruction(): void {}
```

```tspp main.tspp
import existingConstruction from "./library";

const value = fixtureDefaultConstruction;
              ^^^^^^^^^^^^^^^^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.none
```

### Auto-import an overload family once

An exported overload family produces one completion and one import edit.

```tspp library.tspp
export function parseFixture(value: int32): int32 {
    return value;
}

export function parseFixture(value: string): string {
    return value;
}
```

```tspp main.tspp

^ insertion
const value = parseFixtur;
              ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=parseFixture kind=function replace=main.tspp#prefix suffix="(value: int32): int32" description="from ./library" insert="parseFixture(${1:value})$0" snippet=true auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=main.tspp#insertion text="import { parseFixture } from \"./library\";\n"
```

### Omit an import for a visible name

A visible binding wins without a redundant import candidate.

```tspp library.tspp
export function visibleGreeting(): void {}
```

```tspp main.tspp
function visibleGreeting(): void {}

visibleGree
^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=visibleGreeting kind=function replace=main.tspp#prefix suffix="(): void" insert="visibleGreeting()" matches=0,1,2,3,4,5,6,7,8,9,10
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
            "include": ["**/*.tspp"]
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
            "path": "src/*.tspp"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```tspp packages/ui/src/button.tspp
export struct Button {}
```

```tspp packages/app/main.tspp

^ insertion
const value = Button;
              ^^^^^^ prefix
```

```query completion packages/app/main.tspp#prefix@end include_auto_imports=true
@completion.item label=Button kind=struct replace=packages/app/main.tspp#prefix description="from @acme/ui/button" insert="Button {}" auto_import=true matches=0,1,2,3,4,5
@completion.additional_edit item=0 range=packages/app/main.tspp#insertion text="import { Button } from \"@acme/ui/button\";\n"
```

### Auto-import a boundary name match

Auto-import search uses the same ordered name matching as the completion list.

```tspp library.tspp
export function fixtureRenderStatusMessage(): string {
    return "ready";
}
```

```tspp main.tspp

^ insertion
const message = fRSM;
                ^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=fixtureRenderStatusMessage kind=function replace=main.tspp#prefix suffix="(): string" description="from ./library" insert="fixtureRenderStatusMessage()" auto_import=true matches=0,7,13,19
@completion.additional_edit item=0 range=main.tspp#insertion text="import { fixtureRenderStatusMessage } from \"./library\";\n"
```

### Omit an undeclared package export

Public exports require an active direct dependency.

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
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
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
            "path": "src/button.tspp"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```tspp packages/ui/src/button.tspp
export struct Button {}
```

```tspp packages/app/main.tspp
Button
^^^^^^ prefix
```

```query completion packages/app/main.tspp#prefix@end include_auto_imports=true
@completion.none
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
            "include": ["**/*.tspp"]
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
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```tspp packages/ui/src/button.tspp
export struct Button {}
```

```tspp packages/ui/src/button.d.tspp
export type ButtonDeclaration = string;
```

```tspp packages/app/main.tspp
Button
^^^^^^ prefix
```

```query completion packages/app/main.tspp#prefix@end include_auto_imports=true
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
            "include": ["**/*.tspp"]
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
            "path": "src/main.tspp"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```tspp packages/theme/src/main.tspp
export struct FixtureTheme {}
```

```tspp packages/app/main.tspp

^ insertion
const value = FixtureThem;
              ^^^^^^^^^^^ prefix
```

```query completion packages/app/main.tspp#prefix@end include_auto_imports=true
@completion.item label=FixtureTheme kind=struct replace=packages/app/main.tspp#prefix description="from @acme/theme" insert="FixtureTheme {}" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=packages/app/main.tspp#insertion text="import { FixtureTheme } from \"@acme/theme\";\n"
```

### Add a named binding to an existing import

An auto import extends the matching named import declaration.

```tspp library.tspp
export function alpha(): void {}
export function beta(): void {}
```

```tspp main.tspp
import { alpha } from "./library";
              ^ insertion

const value = beta;
              ^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=beta kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="beta()" auto_import=true matches=0,1,2,3
@completion.additional_edit item=0 range=main.tspp#insertion text=", beta"
```

### Add a named binding beside a default import

An auto import adds a named clause after the existing default binding.

```tspp library.tspp
export default function build(): void {}
export function beta(): void {}
```

```tspp main.tspp
import build from "./library";
            ^ default_end

const value = beta;
              ^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=beta kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="beta()" auto_import=true matches=0,1,2,3
@completion.additional_edit item=0 range=main.tspp#default_end text=", { beta }"
```

### Add a default binding beside named imports

An auto import adds the default binding before the existing named clause.

```tspp library.tspp
export default function fixtureBuilder(): void {}
export function value(): void {}
```

```tspp main.tspp
import { value } from "./library";
       ^ insertion

const result = fixtureBuil;
               ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=fixtureBuilder kind=function replace=main.tspp#prefix suffix="(): void" description="from ./library" insert="fixtureBuilder()" auto_import=true matches=0,1,2,3,4,5,6,7,8,9,10
@completion.additional_edit item=0 range=main.tspp#insertion text="fixtureBuilder, "
```

### Insert a new import after an earlier import

An auto import preserves a complete line between consecutive declarations.

```tspp alpha.tspp
export function alpha(): void {}
```

```tspp beta.tspp
export function beta(): void {}
```

```tspp main.tspp
import { alpha } from "./alpha";
                                ^ insertion

const value = beta;
              ^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=beta kind=function replace=main.tspp#prefix suffix="(): void" description="from ./beta" insert="beta()" auto_import=true matches=0,1,2,3
@completion.additional_edit item=0 range=main.tspp#insertion text="\nimport { beta } from \"./beta\";"
```

### Complete an auto import from its current declaration

Auto-import completion reads the declaration selected after each edit.

```tspp library.tspp
export function greet(name: string): string {
    return name;
}
```

```tspp main.tspp

^ insertion
const message = greet;
                ^^^^^ prefix
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=greet kind=function replace=main.tspp#prefix suffix="(name: string): string" description="from ./library" insert="greet(${1:name})$0" snippet=true auto_import=true matches=0,1,2,3,4
@completion.additional_edit item=0 range=main.tspp#insertion text="import { greet } from \"./library\";\n"
```

```diff library.tspp
@@ -1,3 +1,3 @@
-export function greet(name: string): string {
-    return name;
+export function greet(count: int32): int32 {
+    return count;
 }
```

```query completion main.tspp#prefix@end include_auto_imports=true
@completion.item label=greet kind=function replace=main.tspp#prefix suffix="(count: int32): int32" description="from ./library" insert="greet(${1:count})$0" snippet=true auto_import=true matches=0,1,2,3,4
@completion.additional_edit item=0 range=main.tspp#insertion text="import { greet } from \"./library\";\n"
```

## Empty Results

### Return no completion for a string literal

Completion remains suppressed inside string literal contents.

```tspp main.tspp
const value = "gre";
               ^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.none
```

### Return no completion inside a comment

Comments never become expression or statement completion positions.

```tspp main.tspp
// describe the gre value
                ^^^ prefix
const value = 1;
```

```query completion main.tspp#prefix@end
@completion.none
```

### Exclude type declarations from a value position

Value completion does not cross the type namespace.

```tspp main.tspp
type Greeting = string;

const value = Gree;
              ^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.none
```

## Import Paths

### Replace an unfinished module name

Import completion replaces the full module name after the current directory.

```tspp library.tspp
export const value = 1;
```

```tspp main.tspp
import {} from "./libraryWrong";
                  ^^^ prefix
                  ^^^^^^^^^^^^ name
               ^^^^^^^^^^^^^^^^ literal
```

```query completion main.tspp#prefix@end
@completion.item label=library kind=module replace=main.tspp#name matches=0,1,2
```

```query completion main.tspp#literal@start
@completion.none
```

```query completion main.tspp#literal@end
@completion.none
```

### Complete relative modules and folders

Import path completion lists matching modules in the current package.

```tspp utilities/helpers.tspp
export function help(): void {}
```

```tspp user.tspp
export const user = 1;
```

```tspp main.tspp
import {} from "./u";
                  ^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=user kind=module replace=main.tspp#prefix matches=0
@completion.item label=utilities/ kind=folder replace=main.tspp#prefix matches=0
```

### Complete a module inside a folder

Path completion continues within the requested directory.

```tspp utilities/arrays.tspp
export function first(): void {}
```

```tspp utilities/strings.tspp
export function trim(): void {}
```

```tspp main.tspp
import {} from "./utilities/a";
                            ^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=arrays kind=module replace=main.tspp#prefix matches=0
```

### Complete public package paths

Package path completion follows the direct dependency's public exports.

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
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```json packages/ui/destack.json
{
    "name": "@acme/ui",
    "exports": {
        ".": {
            "kind": "module",
            "path": "src/main.tspp"
        },
        "./*": {
            "kind": "module",
            "path": "src/*.tspp"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```tspp packages/ui/src/main.tspp
export const ui = 1;
```

```tspp packages/ui/src/button.tspp
export struct Button {}
```

```tspp packages/ui/src/forms/input.tspp
export struct Input {}
```

```tspp packages/app/main.tspp
import {} from "@acme/u";
                ^^^^^^^ root_prefix

import {} from "@acme/ui/b";
                         ^ button_prefix

import {} from "@acme/ui/f";
                         ^ folder_prefix

import {} from "@acme/ui/forms/i";
                               ^ input_prefix
```

```query completion packages/app/main.tspp#root_prefix@end
@completion.item label=@acme/ui kind=module replace=packages/app/main.tspp#root_prefix matches=0,1,2,3,4,5,6
```

```query completion packages/app/main.tspp#button_prefix@end
@completion.item label=button kind=module replace=packages/app/main.tspp#button_prefix matches=0
```

```query completion packages/app/main.tspp#folder_prefix@end
@completion.item label=forms/ kind=folder replace=packages/app/main.tspp#folder_prefix matches=0
```

```query completion packages/app/main.tspp#input_prefix@end
@completion.item label=input kind=module replace=packages/app/main.tspp#input_prefix matches=0
```

### Complete public builtin paths

Builtin path completion follows the builtin package's public exports.

```tspp main.tspp
import {} from "tspp:conte";
                        ^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=context kind=module replace=main.tspp#prefix matches=0,1,2,3,4
```

### Omit paths from undeclared packages

Package path completion excludes workspace packages outside the active dependency graph.

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
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
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
            "path": "src/button.tspp"
        }
    },
    "targets": {
        "default": {
            "include": ["**/*.tspp"]
        }
    },
    "defaultTarget": "default"
}
```

```tspp packages/ui/src/button.tspp
export struct Button {}
```

```tspp packages/app/main.tspp
import {} from "@acme/u";
                ^^^^^^^ prefix
```

```query completion packages/app/main.tspp#prefix@end
@completion.none
```

## Decorators

### Complete a decorator

Decorator heads complete callable declarations in the active global environment.

```tspp main.tspp
@depre
 ^^^^^ prefix
export function legacy(): void {}
```

```query completion main.tspp#prefix@end
@completion.item label=deprecated kind=constructor replace=main.tspp#prefix suffix="(string): deprecated" insert="deprecated(${1})$0" snippet=true matches=0,1,2,3,4
@completion.item label=deprecated kind=constructor replace=main.tspp#prefix suffix="(): deprecated" insert="deprecated()" matches=0,1,2,3,4
@completion.item label=deprecated kind=constructor replace=main.tspp#prefix suffix="((string,) | ()): deprecated" insert="deprecated(${1})$0" snippet=true matches=0,1,2,3,4
```

## Labels

### Complete an enclosing control label

A control transfer completes labels from its enclosing control targets.

```tspp main.tspp
function choose(): void {
    outer: loop {
        break ou;
              ^^ prefix
    }
}
```

```query completion main.tspp#prefix@end
@completion.item label=outer kind=label replace=main.tspp#prefix matches=0,1
```

## Statements

### Complete a statement keyword

Statement positions include matching keywords.

```tspp main.tspp
function main(): void {
    retur
    ^^^^^ prefix
}
```

```query completion main.tspp#prefix@end
@completion.item label=return kind=keyword replace=main.tspp#prefix matches=0,1,2,3,4
@completion.item label=IteratorReturn kind=struct replace=main.tspp#prefix suffix="<R = void>" insert="IteratorReturn { value: ${1} }$0" snippet=true matches=3,9,10,11,12
```

### Omit statement keywords from an expression

An expression position returns values rather than unrelated statement forms.

```tspp main.tspp
const returnValue = 1;

const result = returnValue;
               ^^^^^^^^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=returnValue kind=constant replace=main.tspp#prefix suffix=": 1" matches=0,1,2,3,4,5,6,7,8,9,10
```

## Primitive Types

### Complete a primitive type

Type positions include matching built-in types.

```tspp main.tspp
declare const value: int3;
                     ^^^^ prefix
```

```query completion main.tspp#prefix@end
@completion.item label=int32 kind=builtin_type replace=main.tspp#prefix matches=0,1,2,3
@completion.item label=uint32 kind=builtin_type replace=main.tspp#prefix matches=1,2,3,4
```
