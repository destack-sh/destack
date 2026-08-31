# Rename

## Local Binding

### Rename a local binding

Renaming a binding updates its definition and references.

```ds main.ds
const count = 1;
      ^^^^^ target
const next = count + count;
```

```query rename main.ds#target new_name=total apply
```

```ds main.ds after
const total = 1;
      ^^^^^ target
const next = total + total;
```

```query rename main.ds#target new_name=amount apply
```

```ds main.ds after
const amount = 1;
      ^^^^^^ target
const next = amount + amount;
```

```query rename main.ds#target new_name=count
```

```ds main.ds after
const count = 1;
const next = count + count;
```

### Rename a function parameter

Renaming a parameter updates the signature and body.

```ds main.ds
function identity(value: int32): int32 {
                  ^^^^^ target
    return value;
}
```

```query rename main.ds#target new_name=result
```

```ds main.ds after
function identity(result: int32): int32 {
    return result;
}
```

### Preserve an object property when renaming its shorthand value

Renaming a local value expands an object shorthand so its property name remains unchanged.

```ds main.ds
const horizontal = 1;
      ^^^^^^^^^^ target

const point = { horizontal };
```

```query rename main.ds#target new_name=x
```

```ds main.ds after
const x = 1;

const point = { horizontal: x };
```

## Type

### Rename a nominal type

Renaming a type updates type and construction references without touching longer names.

```ds main.ds
class Message {}
      ^^^^^^^ target

class MessageFactory {}

function identity(message: Message): Message {
    return message;
}

const created = new Message();
```

```query rename main.ds#target new_name=Packet
```

```ds main.ds after
class Packet {}

class MessageFactory {}

function identity(message: Packet): Packet {
    return message;
}

const created = new Packet();
```

## Field

### Rename a field

Renaming a field updates its declaration and accesses.

```ds main.ds
struct Counter {
    count: int32;
    ^^^^^ target
}

function read(counter: Counter): int32 {
    return counter.count;
}
```

```query rename main.ds#target new_name=value
```

```ds main.ds after
struct Counter {
    value: int32;
}

function read(counter: Counter): int32 {
    return counter.value;
}
```

### Preserve a shorthand value when renaming its field

Renaming a field expands an object shorthand so its local value keeps its original name.

```ds main.ds
struct Point {
    horizontal: int32;
    ^^^^^^^^^^ target
}

const horizontal = 1;
const point = Point { horizontal };
```

```query rename main.ds#target new_name=x
```

```ds main.ds after
struct Point {
    x: int32;
}

const horizontal = 1;
const point = Point { x: horizontal };
```

### Rename a string-keyed field access

A static string key changes with its nominal field.

```ds main.ds
struct Counter {
    count: int32;
    ^^^^^ target
}

function read(counter: Counter): int32 {
    return counter["count"];
}
```

```query rename main.ds#target new_name=value
```

```ds main.ds after
struct Counter {
    value: int32;
}

function read(counter: Counter): int32 {
    return counter["value"];
}
```

### Reject a structural field rename

Structural fields have no declaration identity shared by every compatible shape.

```ds main.ds
type Counter = {
    count: int32,
    ^^^^^ target
};

function read(counter: Counter): int32 {
    return counter.count;
}
```

```query rename main.ds#target new_name=value
@rename.none
```

## Functions

### Rename an exported function

Renaming an export updates its declaration, import, and call.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ target:exported_function
    return name;
}
```

```ds main.ds
import { greet } from "./library.ds";

const message = greet("Destack");
```

```query rename library.ds#target:exported_function new_name=welcome
```

```ds library.ds after
export function welcome(name: string): string {
    return name;
}
```

```ds main.ds after
import { welcome } from "./library.ds";

const message = welcome("Destack");
```

### Preserve a local import alias

Renaming an export leaves its explicit local alias unchanged.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ target:aliased_import
    return name;
}
```

```ds main.ds
import { greet as importedGreet } from "./library.ds";

const greet = 1;
const message = importedGreet("Destack");
```

```query rename library.ds#target:aliased_import new_name=welcome
```

```ds library.ds after
export function welcome(name: string): string {
    return name;
}
```

```ds main.ds after
import { welcome as importedGreet } from "./library.ds";

const greet = 1;
const message = importedGreet("Destack");
```

### Rename a function and query its new name

Hover, definition, reference, and token queries read the applied function name.

```ds main.ds
function greet(name: string): string {
^ declaration:start
         ^^^^^ definition
    return name;
}
^ declaration:end

const message = greet("World");
                ^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 declaration="function greet(name: string): string" location=main.ds#declaration selection=main.ds#definition range=main.ds#reference
```

```query goto_definition main.ds#reference
@goto_definition.target origin=main.ds#reference location=main.ds#declaration selection=main.ds#definition symbol=main.ds#greet@1
```

```query rename main.ds#definition new_name=formatName apply
```

```ds main.ds after
function formatName(name: string): string {
^ declaration:start
         ^^^^^^^^^^ definition
    return name;
}
^ declaration:end

const message = formatName("World");
                ^^^^^^^^^^ reference
```

```query hover main.ds#reference
@hover.item index=0 declaration="function formatName(name: string): string" location=main.ds#declaration selection=main.ds#definition range=main.ds#reference
```

```query goto_definition main.ds#reference
@goto_definition.target origin=main.ds#reference location=main.ds#declaration selection=main.ds#definition symbol=main.ds#formatName@1
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#definition symbol=main.ds#formatName@1
@find_references.reference location=main.ds#reference symbol=main.ds#formatName@1
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#definition type=function modifiers=declaration
@semantic_tokens.token range=main.ds:1:21-1:25 type=parameter modifiers=declaration
@semantic_tokens.token range=main.ds:2:12-2:16 type=parameter
@semantic_tokens.token range=main.ds:5:7-5:14 type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#reference type=function
```

## Namespace Imports

### Rename a namespace member

Renaming an export updates its namespace member accesses.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ target:namespace_import
    return name;
}
```

```ds main.ds
import * as api from "./library.ds";

const message = api.greet("Destack");
```

```query rename library.ds#target:namespace_import new_name=welcome
```

```ds library.ds after
export function welcome(name: string): string {
    return name;
}
```

```ds main.ds after
import * as api from "./library.ds";

const message = api.welcome("Destack");
```

## Re-Exports

### Rename through a named re-export

Renaming an export updates its re-export, downstream import, and call.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ target:named_reexport
    return name;
}
```

```ds barrel.ds
export { greet } from "./library.ds";
```

```ds main.ds
import { greet } from "./barrel.ds";

const message = greet("Destack");
```

```query rename library.ds#target:named_reexport new_name=welcome
```

```ds library.ds after
export function welcome(name: string): string {
    return name;
}
```

```ds barrel.ds after
export { welcome } from "./library.ds";
```

```ds main.ds after
import { welcome } from "./barrel.ds";

const message = welcome("Destack");
```

### Preserve a public re-export alias

Renaming an export leaves its explicit public alias unchanged.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ target:aliased_reexport
    return name;
}
```

```ds barrel.ds
export { greet as hello } from "./library.ds";
```

```ds main.ds
import { hello } from "./barrel.ds";

const message = hello("Destack");
```

```query rename library.ds#target:aliased_reexport new_name=welcome
```

```ds library.ds after
export function welcome(name: string): string {
    return name;
}
```

```ds barrel.ds after
export { welcome as hello } from "./library.ds";
```

## Imported Types

### Rename an exported type

Renaming an exported type updates its import and annotations.

```ds library.ds
export type Settings = {
            ^^^^^^^^ target:exported_type
    enabled: boolean,
};
```

```ds main.ds
import { Settings } from "./library.ds";

const configuration: Settings = { enabled: true };
```

```query rename library.ds#target:exported_type new_name=Configuration
```

```ds library.ds after
export type Configuration = {
    enabled: boolean,
};
```

```ds main.ds after
import { Configuration } from "./library.ds";

const configuration: Configuration = { enabled: true };
```

### Rename a type through a re-export

Renaming an exported type updates its re-export while preserving the public alias.

```ds library.ds
export type Settings = {
            ^^^^^^^^ target:reexported_type
    enabled: boolean,
};
```

```ds barrel.ds
export { Settings as ApplicationSettings } from "./library.ds";
```

```ds main.ds
import { ApplicationSettings } from "./barrel.ds";

const configuration: ApplicationSettings = { enabled: true };
```

```query rename library.ds#target:reexported_type new_name=Configuration
```

```ds library.ds after
export type Configuration = {
    enabled: boolean,
};
```

```ds barrel.ds after
export { Configuration as ApplicationSettings } from "./library.ds";
```

## Methods

### Rename a method

Renaming a method updates its declaration and accesses.

```ds main.ds
class Service {
    run(): void {}
    ^^^ target
}

function start(service: Service): void {
    service.run();
}
```

```query rename main.ds#target new_name=execute
```

```ds main.ds after
class Service {
    execute(): void {}
}

function start(service: Service): void {
    service.execute();
}
```

### Rename an extension method

Renaming an extension method updates its declaration and every extension call.

```ds main.ds
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
    ^^^ target
        return left + right;
    }
}

function total(calculator: Calculator): int32 {
    return calculator.add(1, 2);
}
```

```query rename main.ds#target new_name=sum
```

```ds main.ds after
struct Calculator {}

extension of Calculator {
    sum(left: int32, right: int32): int32 {
        return left + right;
    }
}

function total(calculator: Calculator): int32 {
    return calculator.sum(1, 2);
}
```

### Rename an interface method

Renaming an interface method updates implementations and calls through the interface.

```ds main.ds
interface Renderable {
    render(): string;
    ^^^^^^ target
}

class View implements Renderable {
    render(): string {
        return "";
    }
}

function display(value: Renderable): string {
    return value.render();
}
```

```query rename main.ds#target new_name=draw
```

```ds main.ds after
interface Renderable {
    draw(): string;
}

class View implements Renderable {
    draw(): string {
        return "";
    }
}

function display(value: Renderable): string {
    return value.draw();
}
```

### Rename from an implementing method

Renaming an implementation updates its interface requirement and calls.

```ds library.ds
export interface Renderable {
    render(): string;
}
```

```ds main.ds
import { Renderable } from "./library.ds";

export class View implements Renderable {
    render(): string {
    ^^^^^^ target
        return "";
    }
}

function display(value: View): string {
    return value.render();
}
```

```query rename main.ds#target new_name=draw
```

```ds library.ds after
export interface Renderable {
    draw(): string;
}
```

```ds main.ds after
import { Renderable } from "./library.ds";

export class View implements Renderable {
    draw(): string {
        return "";
    }
}

function display(value: View): string {
    return value.draw();
}
```

## Associated Constants

### Rename an associated constant

Renaming an associated constant updates its declaration and nominal accesses.

```ds main.ds
struct Buffer {
    const Width: uint = 8;
          ^^^^^ target
}

const first = Buffer.Width;
const second = Buffer.Width;
```

```query rename main.ds#target new_name=Size
```

```ds main.ds after
struct Buffer {
    const Size: uint = 8;
}

const first = Buffer.Size;
const second = Buffer.Size;
```

## Associated Types

### Rename an associated type

Renaming an interface associated type updates its implementations and projections.

```ds container.ds
export interface Container {
    type Item;
         ^^^^ target
}
```

```ds main.ds
import { Container } from "./container.ds";

class TextContainer implements Container {
    type Item = string;
}

type Text = TextContainer.Item;
```

```query rename container.ds#target new_name=Element
```

```ds container.ds after
export interface Container {
    type Element;
}
```

```ds main.ds after
import { Container } from "./container.ds";

class TextContainer implements Container {
    type Element = string;
}

type Text = TextContainer.Element;
```

## Enum Members

### Rename an enum member

Renaming a variant updates its declaration and member accesses.

```ds main.ds
enum Color {
    Red,
    ^^^ target
}

const color = Color.Red;
```

```query rename main.ds#target new_name=Crimson
```

```ds main.ds after
enum Color {
    Crimson,
}

const color = Color.Crimson;
```

## Overloads

### Rename an overload family

Renaming one overload updates every declaration and call in its overload family.

```ds main.ds
function parse(value: int32): int32 {
         ^^^^^ target
    return value;
}

function parse(value: string): string {
    return value;
}

const integerValue = parse(1);
const stringValue = parse("one");
```

```query rename main.ds#target new_name=decode
```

```ds main.ds after
function decode(value: int32): int32 {
    return value;
}

function decode(value: string): string {
    return value;
}

const integerValue = decode(1);
const stringValue = decode("one");
```

## Import Aliases

### Rename a local import alias

Renaming an explicit local alias leaves the exported name unchanged.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { greet as welcome } from "./library.ds";
                  ^^^^^^^ target

welcome();
```

```query rename main.ds#target new_name=salute
```

```ds main.ds after
import { greet as salute } from "./library.ds";

salute();
```

### Rename a default import binding

A default import binding is local to the importing module.

```ds library.ds
export default function build(): void {}
```

```ds main.ds
import create from "./library.ds";
       ^^^^^^ target

create();
```

```query rename main.ds#target new_name=construct
```

```ds main.ds after
import construct from "./library.ds";

construct();
```

### Rename a namespace import binding

A namespace import name is local to the importing module.

```ds library.ds
export function ping(): void {}
```

```ds main.ds
import * as api from "./library.ds";
            ^^^ target

api.ping();
```

```query rename main.ds#target new_name=library
```

```ds main.ds after
import * as library from "./library.ds";

library.ping();
```

### Rename an imported type alias

A local alias in the type symbol space changes without renaming the exported type.

```ds library.ds
export type Options = {
    enabled: boolean,
};
```

```ds main.ds
import { Options as Settings } from "./library.ds";
                    ^^^^^^^^ target

declare const settings: Settings;
```

```query rename main.ds#target new_name=Configuration
```

```ds main.ds after
import { Options as Configuration } from "./library.ds";

declare const settings: Configuration;
```

## Default Exports

### Rename a named default export

Renaming a default declaration leaves downstream local import names unchanged.

```ds library.ds
export default function build(): void {}
                        ^^^^^ target
```

```ds main.ds
import create from "./library.ds";

create();
```

```query rename library.ds#target new_name=construct
```

```ds library.ds after
export default function construct(): void {}
```

## Type Parameters

### Rename a function type parameter

Type parameter renames update every reference in the owning declaration.

```ds main.ds
function identity<Value>(value: Value): Value {
                  ^^^^^ target
    return value;
}
```

```query rename main.ds#target new_name=Item
```

```ds main.ds after
function identity<Item>(value: Item): Item {
    return value;
}
```

### Rename a class type parameter

The owning class and all of its members share one type-parameter identity.

```ds main.ds
class Box<Value> {
          ^^^^^ target
    value: Value;

    read(input: Value): Value {
        return input;
    }
}
```

```query rename main.ds#target new_name=Item
```

```ds main.ds after
class Box<Item> {
    value: Item;

    read(input: Item): Item {
        return input;
    }
}
```

### Rename a const value parameter

A const value parameter updates every use in its declaration.

```ds main.ds
type Buffer<const size: usize> = [uint8; size];
                  ^^^^ target
```

```query rename main.ds#target new_name=length
```

```ds main.ds after
type Buffer<const length: usize> = [uint8; length];
```

## Pattern Bindings

### Rename a destructured alias

Renaming a local destructuring alias leaves its property key unchanged.

```ds main.ds
const point = { x: 1 };
const { x: horizontal } = point;
           ^^^^^^^^^^ target
const value = horizontal;
```

```query rename main.ds#target new_name=position
```

```ds main.ds after
const point = { x: 1 };
const { x: position } = point;
const value = position;
```

### Preserve a destructured property when renaming its shorthand binding

Renaming a shorthand binding expands the pattern so its property remains unchanged.

```ds main.ds
declare const point: { horizontal: int32 };

const { horizontal } = point;
        ^^^^^^^^^^ target
const value = horizontal;
```

```query rename main.ds#target new_name=x
```

```ds main.ds after
declare const point: { horizontal: int32 };

const { horizontal: x } = point;
const value = x;
```

### Rename a match binding

A match binding changes only its arm-local declaration and references.

```ds main.ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ target
};
```

```query rename main.ds#target new_name=first
```

```ds main.ds after
declare const pair: (int32, int32);

const total = match (pair) {
    (first, right) => first + right
};
```

## Annotations

### Rename an annotation

Renaming an annotation updates its declaration and applications.

```ds main.ds
newtype tracked = ();
        ^^^^^^^ target

@tracked
class Service {}

@tracked
function start(): void {}
```

```query rename main.ds#target new_name=observed
```

```ds main.ds after
newtype observed = ();

@observed
class Service {}

@observed
function start(): void {}
```

## Calls

### Rename a tagged-template function

Tagged templates and ordinary calls share the function identity.

```ds main.ds
function sql(parts: string[], ...values: int32[]): string {
         ^^^ target
    return "";
}

const query = sql`select ${1}`;
const text = sql([""], 2);
```

```query rename main.ds#target new_name=execute
```

```ds main.ds after
function execute(parts: string[], ...values: int32[]): string {
    return "";
}

const query = execute`select ${1}`;
const text = execute([""], 2);
```

### Rename a function used at const

Const and runtime calls share the function declaration.

```ds main.ds
function build(): int32 {
         ^^^^^ target
    return 1;
}

const first = const build();
const second = build();
```

```query rename main.ds#target new_name=make
```

```ds main.ds after
function make(): int32 {
    return 1;
}

const first = const make();
const second = make();
```

## Labels

### Rename a control label

Renaming a control label updates its declaration and every targeted break.

```ds main.ds
function choose(value: boolean): int32 {
    outer: loop {
    ^^^^^ target
        if (value) {
            break outer: 1;
        }

        break outer: 2;
    }
}
```

```query rename main.ds#target new_name=done
```

```ds main.ds after
function choose(value: boolean): int32 {
    done: loop {
        if (value) {
            break done: 1;
        }

        break done: 2;
    }
}
```

## Using Bindings

### Rename a using binding

A resource binding updates its local references.

```ds main.ds
declare function openSession(): Dispose;

using session = openSession();
      ^^^^^^^ target
const value = session;
```

```query rename main.ds#target new_name=resource
```

```ds main.ds after
declare function openSession(): Dispose;

using resource = openSession();
const value = resource;
```

### Rename an await-using binding

An asynchronous resource binding updates its local references.

```ds main.ds
declare function openResource(): AsyncDispose;

async function run(): void {
    await using resource = openResource();
                ^^^^^^^^ target
    const value = resource;
}
```

```query rename main.ds#target new_name=handle
```

```ds main.ds after
declare function openResource(): AsyncDispose;

async function run(): void {
    await using handle = openResource();
    const value = handle;
}
```

### Rename a for-using binding

A loop resource binding updates references inside its loop body.

```ds main.ds
declare function values(): Dispose[];

for (using item of values()) {
           ^^^^ target
    const next = item;
}
```

```query rename main.ds#target new_name=value
```

```ds main.ds after
declare function values(): Dispose[];

for (using value of values()) {
    const next = value;
}
```

### Rename an exported using binding

An exported resource binding has one declaration and reference identity.

```ds main.ds
declare function openCache(): Dispose;

export using cache = openCache();
             ^^^^^ target
const value = cache;
```

```query rename main.ds#target new_name=store
```

```ds main.ds after
declare function openCache(): Dispose;

export using store = openCache();
const value = store;
```

## No Edit

### Omit a rename to the current name

A rename to the existing name produces no edit.

```ds main.ds
const value = 1;
      ^^^^^ target
const result = value;
```

```query rename main.ds#target new_name=value
@rename.none
```

### Reject a keyword as the new name

A keyword cannot replace an identifier.

```ds main.ds
const value = 1;
      ^^^^^ target
const result = value;
```

```query rename main.ds#target new_name=class
@rename.none
```

### Reject a rename shared by unrelated union members

Renaming one declaration cannot rewrite an occurrence that also belongs to another declaration.

```ds main.ds
class Alpha {
    run(): void {}
    ^^^ target
}

class Beta {
    run(): void {}
}

function start(service: Alpha | Beta): void {
    service.run();
}
```

```query rename main.ds#target new_name=execute
@rename.none
```

## Shadowing

### Rename only one shadowed binding

Distinct lexical bindings remain separate rename identities.

```ds main.ds
const value = 1;

function inner(): int32 {
    const value = 2;
          ^^^^^ target
    return value;
}

const outer = value;
```

```query rename main.ds#target new_name=innerValue
```

```ds main.ds after
const value = 1;

function inner(): int32 {
    const innerValue = 2;
    return innerValue;
}

const outer = value;
```
