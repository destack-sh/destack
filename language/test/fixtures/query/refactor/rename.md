# Rename

## Local Variables

### Rename local variable

Rename should update all occurrences of a symbol to the new name.

```ds
const foo = 1;
//    ^^^ target

const bar = foo + foo;
console.log(foo);
```

`foo` appears 4 times: its definition and 3 uses. Renaming to `baz` should update all 4 occurrences.

```query rename target "baz"
```

```expected:main
const baz = 1;

const bar = baz + baz;
console.log(baz);
```

### Rename function parameter

Renaming a function parameter should update references inside the body.

```ds
function greet(name: string): string {
//             ^^^^ target:param
    return "Hello, " + name;
}
```

```query rename target:param "title"
```

```expected:main
function greet(title: string): string {
    return "Hello, " + title;
}
```

## Classes

### Rename class with type references

Renaming a class should update its definition and all type references.

```ds
class Meetup {
//    ^^^^^^ target
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Meetup = new Meetup();
//       ^^^^^^ ref1
//                   ^^^^^^ ref2
```

Renaming `Meetup` to `Event` should only update `Meetup`, not `MeetupLanguage`.

```query rename target "Event"
```

```expected:main
class Event {
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Event = new Event();
```

### Rename class with field type references

Renaming a class used as a field type should work correctly.

```ds
class Meetup {
//    ^^^^^^ target
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Meetup
//          ^^^^^^ ref
    /// The language preference.
    language: MeetupLanguage
}
```

Renaming `Meetup` to `Event` should only update `Meetup`, not affect `MeetupLanguage`.

```query rename target "Event"
```

```expected:main
class Event {
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Event
    /// The language preference.
    language: MeetupLanguage
}
```

## Interfaces

### Rename interface references

Renaming an interface should update implements clauses and type annotations.

```ds
interface Drawable {
//         ^^^^^^^ target
    function draw(): void;
}

class Sprite implements Drawable {
//                        ^^^^^^^ ref1
    function draw(): void {}
}

const sprite: Drawable = new Sprite();
//             ^^^^^^^ ref2
```

```query rename target "Renderable"
```

```expected:main
interface Renderable {
    function draw(): void;
}

class Sprite implements Renderable {
    function draw(): void {}
}

const sprite: Renderable = new Sprite();
```

### Rename interface members across implementations

Renaming an interface member should update implementations and member accesses.

```ds
interface Drawable {
    draw(): void;
//  ^^^^ target:member
}

class Sprite implements Drawable {
    draw(): void {}
}

function render(item: Drawable): void {
    item.draw();
}
```

```query rename target:member "paint"
```

```expected:main
interface Drawable {
    paint(): void;
}

class Sprite implements Drawable {
    paint(): void {}
}

function render(item: Drawable): void {
    item.paint();
}
```

## Cross-Module

### Rename exported function across imports

Renaming an exported function should update its imports and call sites.

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^^ target
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet } from "./lib.ds";

const message = greet("Destack");
```

```query rename target "salute"
```

```expected:lib
export function salute(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import { salute } from "./lib.ds";

const message = salute("Destack");
```

### Rename interface across modules with re-exports

Renaming an interface should update `export type` re-exports and `import type` references.

```ds:base.ds
export interface Drawable {
//                 ^^^^^^^ target
    function draw(): void;
}
```

```ds:public.ds
export type { Drawable } from "./base.ds";
```

```ds:main.ds
import type { Drawable } from "./public.ds";

class Sprite implements Drawable {
    function draw(): void {}
}

const sprite: Drawable = new Sprite();
```

```query rename target "Renderable"
```

```expected:base
export interface Renderable {
    function draw(): void;
}
```

```expected:public
export type { Renderable } from "./base.ds";
```

```expected:main
import type { Renderable } from "./public.ds";

class Sprite implements Renderable {
    function draw(): void {}
}

const sprite: Renderable = new Sprite();
```

### Rename default export across imports

Renaming a default export should update the export name and default import usage.

```ds:lib_default.ds
export default function greet(name: string): string {
//                      ^^^^^ target:default
    return "Hello, " + name;
}
```

```ds:main_default.ds
import greet from "./lib_default.ds";

const message = greet("Destack");
```

```query rename target:default "salute"
```

```expected:lib_default
export default function salute(name: string): string {
    return "Hello, " + name;
}
```

```expected:main_default
import salute from "./lib_default.ds";

const message = salute("Destack");
```

### Rename default exported class across imports

Renaming a default exported class should update the class name and default import usage.

```ds:widget.ds
export default class Widget {
//                   ^^^^^^ target:default-class
    value: int32;
}
```

```ds:main_widget.ds
import Widget from "./widget.ds";

const widget = new Widget();
```

```query rename target:default-class "Gadget"
```

```expected:widget
export default class Gadget {
    value: int32;
}
```

```expected:main_widget
import Gadget from "./widget.ds";

const widget = new Gadget();
```

### Rename default exported interface across imports

Renaming a default exported interface should update the interface name and default import usage.

```ds:port.ds
export default interface Port {
//                       ^^^^ target:default-interface
    function open(): void;
}
```

```ds:main_port.ds
import Port from "./port.ds";

class Socket implements Port {
    function open(): void {}
}
```

```query rename target:default-interface "Channel"
```

```expected:port
export default interface Channel {
    function open(): void;
}
```

```expected:main_port
import Channel from "./port.ds";

class Socket implements Channel {
    function open(): void {}
}
```

## Members

### Rename class method

Renaming a class method should update the definition and all call sites.

```ds
class Counter {
    inc(): int32 {
//  ^^^ target
        return 1;
    }
}

function main() {
    const counter = new Counter();
    const first = counter.inc();
    const second = counter.inc();
}
```

Renaming `inc` to `next` should update all method calls.

```query rename target "next"
```

```expected:main
class Counter {
    next(): int32 {
        return 1;
    }
}

function main() {
    const counter = new Counter();
    const first = counter.next();
    const second = counter.next();
}
```

### Rename struct field

Renaming a struct field should update the definition and field accesses.

```ds
struct Point {
    x: int32
//  ^ target
    y: int32
}

function length(p: Point): int32 {
    return p.x + p.x;
}
```

Renaming `x` to `z` should update all field accesses.

```query rename target "z"
```

```expected:main
struct Point {
    z: int32
    y: int32
}

function length(p: Point): int32 {
    return p.z + p.z;
}
```

### Rename class field

Renaming a class field should update the definition and member accesses.

```ds
class Counter {
    value: int32;
//  ^^^^^ target
    inc(): int32 {
        return this.value + 1;
    }
}

function main() {
    const counter = new Counter();
    const total = counter.value + counter.value;
}
```

```query rename target "total"
```

```expected:main
class Counter {
    total: int32;
    inc(): int32 {
        return this.total + 1;
    }
}

function main() {
    const counter = new Counter();
    const total = counter.total + counter.total;
}
```

## Enums

### Rename enum type

Renaming an enum type should update the declaration name and type references.

```ds
enum Status {
//   ^^^^^^ target:enum
    Pending,
    Active,
}

const state: Status = Status.Pending;
```

```query rename target:enum "State"
```

```expected:main
enum State {
    Pending,
    Active,
}

const state: State = State.Pending;
```

### Rename enum member

Renaming an enum member should update the definition and all uses.

```ds
enum Status {
    Pending,
//  ^^^^^^ target
    Active,
}

const state = Status.Pending;
//                     ^ use:pending
```

```query rename target "Queued"
```

```expected:main
enum Status {
    Queued,
    Active,
}

const state = Status.Queued;
```

## Namespaces

### Rename namespace

Renaming a namespace should update the declaration and all member accesses.

```ds
namespace Api {
//        ^^^ target:namespace
    export function greet(): string {
        return "Hello";
    }
}

const message = Api.greet();
```

```query rename target:namespace "Service"
```

```expected:main
namespace Service {
    export function greet(): string {
        return "Hello";
    }
}

const message = Service.greet();
```

### Rename namespace member

Renaming a namespace member should update the declaration and member accesses.

```ds
namespace Api {
    export function greet(): string {
//                  ^^^^^ target:namespace-member
        return "Hello";
    }
}

const message = Api.greet();
```

```query rename target:namespace-member "welcome"
```

```expected:main
namespace Api {
    export function welcome(): string {
        return "Hello";
    }
}

const message = Api.welcome();
```

## Scope and Shadowing

### Rename inner binding without touching outer binding

Renaming a shadowed binding should only affect the selected symbol and its references.

```ds
const value = 1;
//    ^^^^^ def:outer

function main(): int32 {
    const value = 2;
//        ^^^^^ target:inner

    return value + 1;
//         ^^^^^ use:inner
}

const total = value + 3;
//            ^^^^^ use:outer
```

Renaming the inner `value` to `innerValue` should not change the outer `value`.

```query rename target:inner "innerValue"
```

```expected:main
const value = 1;

function main(): int32 {
    const innerValue = 2;

    return innerValue + 1;
}

const total = value + 3;
```

## Import Shapes

### Rename type alias used via type-only import

Renaming a type alias should update type-only imports and type usages.

```ds:types.ds
export type Options = {
//          ^^^^^^^ target:type
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(options: Options): void {
    console.log(options.name);
}
```

Renaming `Options` to `Config` should update the export, import, and usage.

```query rename target:type "Config"
```

```expected:types
export type Config = {
    name: string,
};
```

```expected:main
import type { Config } from "./types.ds";

function configure(options: Config): void {
    console.log(options.name);
}
```

### Rename newtype across imports

Renaming a newtype should update type-only imports and constructor calls.

```ds:types_newtype.ds
export newtype UserId = int64;
//               ^^^^^^ target:newtype
```

```ds:main_newtype.ds
import type { UserId } from "./types_newtype.ds";

const user_id: UserId = UserId(42);
```

```query rename target:newtype "AccountId"
```

```expected:types_newtype
export newtype AccountId = int64;
```

```expected:main_newtype
import type { AccountId } from "./types_newtype.ds";

const user_id: AccountId = AccountId(42);
```

### Rename exported symbol imported with alias

Renaming an exported symbol should update the imported name but keep the local alias.

```ds:lib.ds
export function greet(name: string): string {
//              ^^^^^ target:alias
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet as localGreet } from "./lib.ds";

const message = localGreet("Destack");
```

Renaming `greet` to `welcome` should only change the exported and imported names.

```query rename target:alias "welcome"
```

```expected:lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import { welcome as localGreet } from "./lib.ds";

const message = localGreet("Destack");
```

### Rename exported symbol accessed via namespace import

Renaming an exported symbol should update namespace member accesses.

```ds:api.ds
export function greet(name: string): string {
//              ^^^^^ target:ns
    return "Hello, " + name;
}
```

```ds:main.ds
import * as api from "./api.ds";

const message = api.greet("Destack");
```

Renaming `greet` to `welcome` should update the namespace member access.

```query rename target:ns "welcome"
```

```expected:api
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:main
import * as api from "./api.ds";

const message = api.welcome("Destack");
```

## Type Parameters

### Rename function type parameter

Renaming a type parameter should update all references in the signature and body.

```ds
function wrap<T>(value: T): T {
//             ^ target:typeparam
    const current: T = value;
    return current;
}
```

```query rename target:typeparam "U"
```

```expected:main
function wrap<U>(value: U): U {
    const current: U = value;
    return current;
}
```

### Rename class type parameter

Renaming a class type parameter should update references in the class body.

```ds
class Box<T> {
//        ^ target:class-param
    value: T;
}

const box: Box<int32>;
```

```query rename target:class-param "U"
```

```expected:main
class Box<U> {
    value: U;
}

const box: Box<int32>;
```

### Rename interface type parameter

Renaming an interface type parameter should update references in the interface body.

```ds
interface Store<T> {
//              ^ target:interface-param
    get(): T;
}

class Cache implements Store<int32> {
    get(): int32 {
        return 1;
    }
}
```

```query rename target:interface-param "U"
```

```expected:main
interface Store<U> {
    get(): U;
}

class Cache implements Store<int32> {
    get(): int32 {
        return 1;
    }
}
```

## Destructuring

### Rename destructured binding

Renaming a binding introduced by destructuring should update its usages.

```ds
const config = { value: 1 };
const { value } = config;
//      ^^^^^ target:destructure

const total = value + value;
```

```query rename target:destructure "count"
```

```expected:main
const config = { value: 1 };
const { count } = config;

const total = count + count;
```

### Rename destructured binding with alias

Renaming a binding with a destructuring alias should preserve the property name.

```ds
const config = { value: 1 };
const { value: current } = config;
//             ^^^^^^^ target:alias

const total = current + 1;
```

```query rename target:alias "amount"
```

```expected:main
const config = { value: 1 };
const { value: amount } = config;

const total = amount + 1;
```
