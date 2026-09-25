# Rename

## Local Binding

### Rename a local binding

Renaming a binding updates its definition and references.

```tspp main.tspp
const count = 1;
      ^^^^^ target
const next = count + count;
```

```query rename main.tspp#target new_name=total apply
```

```tspp main.tspp after
const total = 1;
      ^^^^^ target
const next = total + total;
```

```query rename main.tspp#target new_name=amount apply
```

```tspp main.tspp after
const amount = 1;
      ^^^^^^ target
const next = amount + amount;
```

```query rename main.tspp#target new_name=count
```

```tspp main.tspp after
const count = 1;
const next = count + count;
```

### Rename a function parameter

Renaming a parameter updates the signature and body.

```tspp main.tspp
function identity(value: int32): int32 {
                  ^^^^^ target
    return value;
}
```

```query rename main.tspp#target new_name=result
```

```tspp main.tspp after
function identity(result: int32): int32 {
    return result;
}
```

### Preserve an object property when renaming its shorthand value

Renaming a local value expands an object shorthand so its property name remains unchanged.

```tspp main.tspp
const horizontal = 1;
      ^^^^^^^^^^ target

const point = { horizontal };
```

```query rename main.tspp#target new_name=x
```

```tspp main.tspp after
const x = 1;

const point = { horizontal: x };
```

## Type

### Rename a nominal type

Renaming a type updates type and construction references without touching longer names.

```tspp main.tspp
class Message {}
      ^^^^^^^ target

class MessageFactory {}

function identity(message: Message): Message {
    return message;
}

const created = new Message();
```

```query rename main.tspp#target new_name=Packet
```

```tspp main.tspp after
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

```tspp main.tspp
struct Counter {
    count: int32;
    ^^^^^ target
}

function read(counter: Counter): int32 {
    return counter.count;
}
```

```query rename main.tspp#target new_name=value
```

```tspp main.tspp after
struct Counter {
    value: int32;
}

function read(counter: Counter): int32 {
    return counter.value;
}
```

### Preserve a shorthand value when renaming its field

Renaming a field expands an object shorthand so its local value keeps its original name.

```tspp main.tspp
struct Point {
    horizontal: int32;
    ^^^^^^^^^^ target
}

const horizontal = 1;
const point = Point { horizontal };
```

```query rename main.tspp#target new_name=x
```

```tspp main.tspp after
struct Point {
    x: int32;
}

const horizontal = 1;
const point = Point { x: horizontal };
```

### Rename a string-keyed field access

A static string key changes with its nominal field.

```tspp main.tspp
struct Counter {
    count: int32;
    ^^^^^ target
}

function read(counter: Counter): int32 {
    return counter["count"];
}
```

```query rename main.tspp#target new_name=value
```

```tspp main.tspp after
struct Counter {
    value: int32;
}

function read(counter: Counter): int32 {
    return counter["value"];
}
```

### Reject a structural field rename

Structural fields have no declaration identity shared by every compatible shape.

```tspp main.tspp
type Counter = {
    count: int32,
    ^^^^^ target
};

function read(counter: Counter): int32 {
    return counter.count;
}
```

```query rename main.tspp#target new_name=value
@rename.none
```

## Functions

### Rename an exported function

Renaming an export updates its declaration, import, and call.

```tspp library.tspp
export function greet(name: string): string {
                ^^^^^ target:exported_function
    return name;
}
```

```tspp main.tspp
import { greet } from "./library.tspp";

const message = greet("Destack");
```

```query rename library.tspp#target:exported_function new_name=welcome
```

```tspp library.tspp after
export function welcome(name: string): string {
    return name;
}
```

```tspp main.tspp after
import { welcome } from "./library.tspp";

const message = welcome("Destack");
```

### Preserve a local import alias

Renaming an export leaves its explicit local alias unchanged.

```tspp library.tspp
export function greet(name: string): string {
                ^^^^^ target:aliased_import
    return name;
}
```

```tspp main.tspp
import { greet as importedGreet } from "./library.tspp";

const greet = 1;
const message = importedGreet("Destack");
```

```query rename library.tspp#target:aliased_import new_name=welcome
```

```tspp library.tspp after
export function welcome(name: string): string {
    return name;
}
```

```tspp main.tspp after
import { welcome as importedGreet } from "./library.tspp";

const greet = 1;
const message = importedGreet("Destack");
```

### Rename a function and use its new name

Hover, definition, references, and highlighting use the applied function name.

```tspp main.tspp
function greet(name: string): string {
^ declaration:start
         ^^^^^ definition
    return name;
}
^ declaration:end

const message = greet("World");
                ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function greet(name: string): string" location=main.tspp#declaration selection=main.tspp#definition range=main.tspp#reference
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=main.tspp#declaration selection=main.tspp#definition symbol=main.tspp#greet@1
```

```query rename main.tspp#definition new_name=formatName apply
```

```tspp main.tspp after
function formatName(name: string): string {
^ declaration:start
         ^^^^^^^^^^ definition
    return name;
}
^ declaration:end

const message = formatName("World");
                ^^^^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function formatName(name: string): string" location=main.tspp#declaration selection=main.tspp#definition range=main.tspp#reference
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=main.tspp#declaration selection=main.tspp#definition symbol=main.tspp#formatName@1
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#definition symbol=main.tspp#formatName@1
@find_references.reference location=main.tspp#reference symbol=main.tspp#formatName@1
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#definition type=function modifiers=declaration
@semantic_tokens.token range=main.tspp:1:21-1:25 type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp:2:12-2:16 type=parameter
@semantic_tokens.token range=main.tspp:5:7-5:14 type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#reference type=function
```

## Namespace Imports

### Rename a namespace member

Renaming an export updates its namespace member accesses.

```tspp library.tspp
export function greet(name: string): string {
                ^^^^^ target:namespace_import
    return name;
}
```

```tspp main.tspp
import * as api from "./library.tspp";

const message = api.greet("Destack");
```

```query rename library.tspp#target:namespace_import new_name=welcome
```

```tspp library.tspp after
export function welcome(name: string): string {
    return name;
}
```

```tspp main.tspp after
import * as api from "./library.tspp";

const message = api.welcome("Destack");
```

## Re-Exports

### Rename through a named re-export

Renaming an export updates its re-export, downstream import, and call.

```tspp library.tspp
export function greet(name: string): string {
                ^^^^^ target:named_reexport
    return name;
}
```

```tspp barrel.tspp
export { greet } from "./library.tspp";
```

```tspp main.tspp
import { greet } from "./barrel.tspp";

const message = greet("Destack");
```

```query rename library.tspp#target:named_reexport new_name=welcome
```

```tspp library.tspp after
export function welcome(name: string): string {
    return name;
}
```

```tspp barrel.tspp after
export { welcome } from "./library.tspp";
```

```tspp main.tspp after
import { welcome } from "./barrel.tspp";

const message = welcome("Destack");
```

### Preserve a public re-export alias

Renaming an export leaves its explicit public alias unchanged.

```tspp library.tspp
export function greet(name: string): string {
                ^^^^^ target:aliased_reexport
    return name;
}
```

```tspp barrel.tspp
export { greet as hello } from "./library.tspp";
```

```tspp main.tspp
import { hello } from "./barrel.tspp";

const message = hello("Destack");
```

```query rename library.tspp#target:aliased_reexport new_name=welcome
```

```tspp library.tspp after
export function welcome(name: string): string {
    return name;
}
```

```tspp barrel.tspp after
export { welcome as hello } from "./library.tspp";
```

## Imported Types

### Rename an exported type

Renaming an exported type updates its import and annotations.

```tspp library.tspp
export type Settings = {
            ^^^^^^^^ target:exported_type
    enabled: boolean,
};
```

```tspp main.tspp
import { Settings } from "./library.tspp";

const configuration: Settings = { enabled: true };
```

```query rename library.tspp#target:exported_type new_name=Configuration
```

```tspp library.tspp after
export type Configuration = {
    enabled: boolean,
};
```

```tspp main.tspp after
import { Configuration } from "./library.tspp";

const configuration: Configuration = { enabled: true };
```

### Rename a type through a re-export

Renaming an exported type updates its re-export while preserving the public alias.

```tspp library.tspp
export type Settings = {
            ^^^^^^^^ target:reexported_type
    enabled: boolean,
};
```

```tspp barrel.tspp
export { Settings as ApplicationSettings } from "./library.tspp";
```

```tspp main.tspp
import { ApplicationSettings } from "./barrel.tspp";

const configuration: ApplicationSettings = { enabled: true };
```

```query rename library.tspp#target:reexported_type new_name=Configuration
```

```tspp library.tspp after
export type Configuration = {
    enabled: boolean,
};
```

```tspp barrel.tspp after
export { Configuration as ApplicationSettings } from "./library.tspp";
```

## Methods

### Rename a method

Renaming a method updates its declaration and accesses.

```tspp main.tspp
class Service {
    run(): void {}
    ^^^ target
}

function start(service: Service): void {
    service.run();
}
```

```query rename main.tspp#target new_name=execute
```

```tspp main.tspp after
class Service {
    execute(): void {}
}

function start(service: Service): void {
    service.execute();
}
```

### Rename an extension method

Renaming an extension method updates its declaration and every extension call.

```tspp main.tspp
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

```query rename main.tspp#target new_name=sum
```

```tspp main.tspp after
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

```tspp main.tspp
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

```query rename main.tspp#target new_name=draw
```

```tspp main.tspp after
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

```tspp library.tspp
export interface Renderable {
    render(): string;
}
```

```tspp main.tspp
import { Renderable } from "./library.tspp";

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

```query rename main.tspp#target new_name=draw
```

```tspp library.tspp after
export interface Renderable {
    draw(): string;
}
```

```tspp main.tspp after
import { Renderable } from "./library.tspp";

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

```tspp main.tspp
struct Buffer {
    const Width: uint = 8;
          ^^^^^ target
}

const first = Buffer.Width;
const second = Buffer.Width;
```

```query rename main.tspp#target new_name=Size
```

```tspp main.tspp after
struct Buffer {
    const Size: uint = 8;
}

const first = Buffer.Size;
const second = Buffer.Size;
```

## Associated Types

### Rename an associated type

Renaming an interface associated type updates its implementations and projections.

```tspp container.tspp
export interface Container {
    type Item;
         ^^^^ target
}
```

```tspp main.tspp
import { Container } from "./container.tspp";

class TextContainer implements Container {
    type Item = string;
}

type Text = TextContainer.Item;
```

```query rename container.tspp#target new_name=Element
```

```tspp container.tspp after
export interface Container {
    type Element;
}
```

```tspp main.tspp after
import { Container } from "./container.tspp";

class TextContainer implements Container {
    type Element = string;
}

type Text = TextContainer.Element;
```

## Enum Members

### Rename an enum member

Renaming a variant updates its declaration and member accesses.

```tspp main.tspp
enum Color {
    Red,
    ^^^ target
}

const color = Color.Red;
```

```query rename main.tspp#target new_name=Crimson
```

```tspp main.tspp after
enum Color {
    Crimson,
}

const color = Color.Crimson;
```

## Overloads

### Rename an overload family

Renaming one overload updates every declaration and call in its overload family.

```tspp main.tspp
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

```query rename main.tspp#target new_name=decode
```

```tspp main.tspp after
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

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";
                  ^^^^^^^ target

welcome();
```

```query rename main.tspp#target new_name=salute
```

```tspp main.tspp after
import { greet as salute } from "./library.tspp";

salute();
```

### Rename a default import binding

A default import binding is local to the importing module.

```tspp library.tspp
export default function build(): void {}
```

```tspp main.tspp
import create from "./library.tspp";
       ^^^^^^ target

create();
```

```query rename main.tspp#target new_name=construct
```

```tspp main.tspp after
import construct from "./library.tspp";

construct();
```

### Rename a namespace import binding

A namespace import name is local to the importing module.

```tspp library.tspp
export function ping(): void {}
```

```tspp main.tspp
import * as api from "./library.tspp";
            ^^^ target

api.ping();
```

```query rename main.tspp#target new_name=library
```

```tspp main.tspp after
import * as library from "./library.tspp";

library.ping();
```

### Rename a nested namespace in type paths

Renaming a namespace re-export updates the exact namespace segment in each type path.

```tspp model.tspp
export struct Packet {}
```

```tspp library.tspp
export * as models from "./model";
```

```tspp main.tspp
import * as library from "./library";

declare const packet: library.models.Packet;
                              ^^^^^^ target
```

```query rename main.tspp#target new_name=types
```

```tspp library.tspp after
export * as types from "./model";
```

```tspp main.tspp after
import * as library from "./library";

declare const packet: library.types.Packet;
```

### Rename an imported type alias

A local alias in the type symbol space changes without renaming the exported type.

```tspp library.tspp
export type Options = {
    enabled: boolean,
};
```

```tspp main.tspp
import { Options as Settings } from "./library.tspp";
                    ^^^^^^^^ target

declare const settings: Settings;
```

```query rename main.tspp#target new_name=Configuration
```

```tspp main.tspp after
import { Options as Configuration } from "./library.tspp";

declare const settings: Configuration;
```

## Default Exports

### Rename a named default export

Renaming a default declaration leaves downstream local import names unchanged.

```tspp library.tspp
export default function build(): void {}
                        ^^^^^ target
```

```tspp main.tspp
import create from "./library.tspp";

create();
```

```query rename library.tspp#target new_name=construct
```

```tspp library.tspp after
export default function construct(): void {}
```

## Type Parameters

### Rename a function type parameter

Type parameter renames update every reference in the owning declaration.

```tspp main.tspp
function identity<Value>(value: Value): Value {
                  ^^^^^ target
    return value;
}
```

```query rename main.tspp#target new_name=Item
```

```tspp main.tspp after
function identity<Item>(value: Item): Item {
    return value;
}
```

### Rename a class type parameter

The owning class and all of its members share one type-parameter identity.

```tspp main.tspp
class Box<Value> {
          ^^^^^ target
    value: Value;

    read(input: Value): Value {
        return input;
    }
}
```

```query rename main.tspp#target new_name=Item
```

```tspp main.tspp after
class Box<Item> {
    value: Item;

    read(input: Item): Item {
        return input;
    }
}
```

### Rename a const value parameter

A const value parameter updates every use in its declaration.

```tspp main.tspp
type Buffer<const size: usize> = [uint8; size];
                  ^^^^ target
```

```query rename main.tspp#target new_name=length
```

```tspp main.tspp after
type Buffer<const length: usize> = [uint8; length];
```

## Pattern Bindings

### Rename a destructured alias

Renaming a local destructuring alias leaves its property key unchanged.

```tspp main.tspp
const point = { x: 1 };
const { x: horizontal } = point;
           ^^^^^^^^^^ target
const value = horizontal;
```

```query rename main.tspp#target new_name=position
```

```tspp main.tspp after
const point = { x: 1 };
const { x: position } = point;
const value = position;
```

### Preserve a destructured property when renaming its shorthand binding

Renaming a shorthand binding expands the pattern so its property remains unchanged.

```tspp main.tspp
declare const point: { horizontal: int32 };

const { horizontal } = point;
        ^^^^^^^^^^ target
const value = horizontal;
```

```query rename main.tspp#target new_name=x
```

```tspp main.tspp after
declare const point: { horizontal: int32 };

const { horizontal: x } = point;
const value = x;
```

### Rename a match binding

A match binding changes only its arm-local declaration and references.

```tspp main.tspp
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ target
};
```

```query rename main.tspp#target new_name=first
```

```tspp main.tspp after
declare const pair: (int32, int32);

const total = match (pair) {
    (first, right) => first + right
};
```

## Annotations

### Rename an annotation

Renaming an annotation updates its declaration and applications.

```tspp main.tspp
newtype tracked = ();
        ^^^^^^^ target

@tracked
class Service {}

@tracked
function start(): void {}
```

```query rename main.tspp#target new_name=observed
```

```tspp main.tspp after
newtype observed = ();

@observed
class Service {}

@observed
function start(): void {}
```

## Calls

### Rename a tagged-template function

Tagged templates and ordinary calls share the function identity.

```tspp main.tspp
function sql(parts: string[], ...values: int32[]): string {
         ^^^ target
    return "";
}

const query = sql`select ${1}`;
const text = sql([""], 2);
```

```query rename main.tspp#target new_name=execute
```

```tspp main.tspp after
function execute(parts: string[], ...values: int32[]): string {
    return "";
}

const query = execute`select ${1}`;
const text = execute([""], 2);
```

### Rename a function used at const

Const and runtime calls share the function declaration.

```tspp main.tspp
function build(): int32 {
         ^^^^^ target
    return 1;
}

const first = const build();
const second = build();
```

```query rename main.tspp#target new_name=make
```

```tspp main.tspp after
function make(): int32 {
    return 1;
}

const first = const make();
const second = make();
```

## Labels

### Rename a control label

Renaming a control label updates its declaration and every targeted break.

```tspp main.tspp
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

```query rename main.tspp#target new_name=done
```

```tspp main.tspp after
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

```tspp main.tspp
declare function openSession(): Dispose;

using session = openSession();
      ^^^^^^^ target
const value = session;
```

```query rename main.tspp#target new_name=resource
```

```tspp main.tspp after
declare function openSession(): Dispose;

using resource = openSession();
const value = resource;
```

### Rename an await-using binding

An asynchronous resource binding updates its local references.

```tspp main.tspp
declare function openResource(): AsyncDispose;

async function run(): void {
    await using resource = openResource();
                ^^^^^^^^ target
    const value = resource;
}
```

```query rename main.tspp#target new_name=handle
```

```tspp main.tspp after
declare function openResource(): AsyncDispose;

async function run(): void {
    await using handle = openResource();
    const value = handle;
}
```

### Rename a for-using binding

A loop resource binding updates references inside its loop body.

```tspp main.tspp
declare function values(): Dispose[];

for (using item of values()) {
           ^^^^ target
    const next = item;
}
```

```query rename main.tspp#target new_name=value
```

```tspp main.tspp after
declare function values(): Dispose[];

for (using value of values()) {
    const next = value;
}
```

### Rename an exported using binding

An exported resource binding has one declaration and reference identity.

```tspp main.tspp
declare function openCache(): Dispose;

export using cache = openCache();
             ^^^^^ target
const value = cache;
```

```query rename main.tspp#target new_name=store
```

```tspp main.tspp after
declare function openCache(): Dispose;

export using store = openCache();
const value = store;
```

## No Edit

### Omit a rename to the current name

A rename to the existing name produces no edit.

```tspp main.tspp
const value = 1;
      ^^^^^ target
const result = value;
```

```query rename main.tspp#target new_name=value
@rename.none
```

### Reject a keyword as the new name

A keyword cannot replace an identifier.

```tspp main.tspp
const value = 1;
      ^^^^^ target
const result = value;
```

```query rename main.tspp#target new_name=class
@rename.none
```

### Reject a rename shared by unrelated union members

Renaming one declaration cannot rewrite an occurrence that also belongs to another declaration.

```tspp main.tspp
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

```query rename main.tspp#target new_name=execute
@rename.none
```

## Shadowing

### Rename only one shadowed binding

Distinct lexical bindings remain separate rename identities.

```tspp main.tspp
const value = 1;

function inner(): int32 {
    const value = 2;
          ^^^^^ target
    return value;
}

const outer = value;
```

```query rename main.tspp#target new_name=innerValue
```

```tspp main.tspp after
const value = 1;

function inner(): int32 {
    const innerValue = 2;
    return innerValue;
}

const outer = value;
```
