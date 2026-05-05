# Rename

## Local Variables

### Rename local variable

Rename should update all occurrences of a symbol to the new name.

```ds
const foo = 1;
//      ^^^ target

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
//               ^^^^ target:param
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
//      ^^^^^^ target
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Meetup = new Meetup();
//         ^^^^^^ ref1
//                      ^^^^^^ ref2
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
//      ^^^^^^ target
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Meetup
//            ^^^^^^ ref
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
//          ^^^^^^^^ target
    function draw(): void;
}

class Sprite implements Drawable {
//                        ^^^^^^^^ ref1
    function draw(): void {}
}

const sprite: Drawable = new Sprite();
//              ^^^^^^^^ ref2
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
//    ^^^^ target:member
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
//                ^^^^^ target
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
//                 ^^^^^^^^ target
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
//                        ^^^^^ target:default
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
//                     ^^^^^^ target:default-class
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
//                         ^^^^ target:default-interface
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
//    ^^^ target
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
//    ^ target
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
//    ^^^^^ target
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
//     ^^^^^^ target:enum
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
//    ^^^^^^^ target
    Active,
}

const state = Status.Pending;
//                     ^^^^^^^ use:pending
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
//          ^^^ target:namespace
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
//                    ^^^^^ target:namespace-member
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

### Rename namespace member without touching shadowed locals

Renaming a namespace member should not update local object properties with the same receiver and member names.

```ds
namespace Api {
    export function greet(): string {
//                    ^^^^^ target:namespace-member-shadow
        return "Hello";
    }
}

function localMessage(): string {
    const Api = {
        greet: function(): string {
            return "Local";
        },
    };

    return Api.greet();
}

const message = Api.greet();
```

```query rename target:namespace-member-shadow "welcome"
```

```expected:main
namespace Api {
    export function welcome(): string {
        return "Hello";
    }
}

function localMessage(): string {
    const Api = {
        greet: function(): string {
            return "Local";
        },
    };

    return Api.greet();
}

const message = Api.welcome();
```

### Rename namespace with shadowed local binding

Renaming a namespace should not touch shadowed local values with the same name.

```ds
namespace Api {
//          ^^^ target:namespace-shadow
    export function greet(): string {
        return "Hello";
    }
}

function localMessage(): string {
    const Api = {
        greet: function(): string {
            return "Local";
        },
    };

    return Api.greet();
}

const message = Api.greet();
```

```query rename target:namespace-shadow "Service"
```

```expected:main
namespace Service {
    export function greet(): string {
        return "Hello";
    }
}

function localMessage(): string {
    const Api = {
        greet: function(): string {
            return "Local";
        },
    };

    return Api.greet();
}

const message = Service.greet();
```

### Rename namespace in parenthesized member access

Renaming a namespace should update parenthesized receiver member accesses.

```ds
namespace Api {
//          ^^^ target:namespace-paren
    export function greet(): string {
        return "Hello";
    }
}

const message = (Api).greet();
```

```query rename target:namespace-paren "Service"
```

```expected:main
namespace Service {
    export function greet(): string {
        return "Hello";
    }
}

const message = (Service).greet();
```

## Scope and Shadowing

### Rename inner binding without touching outer binding

Renaming a shadowed binding should only affect the selected symbol and its references.

```ds
const value = 1;
//      ^^^^^ def:outer

function main(): int32 {
    const value = 2;
//          ^^^^^ target:inner

    return value + 1;
//           ^^^^^ use:inner
}

const total = value + 3;
//              ^^^^^ use:outer
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
//            ^^^^^^^ target:type
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
//                ^^^^^ target:alias
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

### Rename local import alias

Renaming a local import alias should update only the alias declaration and local uses.

```ds:alias_local_lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_local_main.ds
import { greet as localGreet } from "./alias_local_lib.ds";

const message = localGreet("Destack");
//                ^^^^^^^^^^ target:local_alias
```

```query rename target:local_alias "sayHello"
```

```expected:alias_local_lib
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```expected:alias_local_main
import { greet as sayHello } from "./alias_local_lib.ds";

const message = sayHello("Destack");
```

### Rename imported name in aliased import

Renaming the imported name in an aliased import should update the exported symbol and imported specifier, but keep the local alias.

```ds:alias_imported_lib.ds
export function greet(name: string): string {
//                ^^^^^ target:imported_name
    return "Hello, " + name;
}
```

```ds:alias_imported_main.ds
import { greet as localGreet } from "./alias_imported_lib.ds";
//         ^^^^^ use:imported_name

const message = localGreet("Destack");
```

```query rename use:imported_name "welcome"
```

```expected:alias_imported_lib
export function welcome(name: string): string {
    return "Hello, " + name;
}
```

```expected:alias_imported_main
import { welcome as localGreet } from "./alias_imported_lib.ds";

const message = localGreet("Destack");
```

### Rename namespace import alias

Renaming a namespace import alias should update local alias bindings and local receiver uses only.

```ds:namespace_alias_lib.ds
export function ping(): void {}
```

```ds:namespace_alias_main.ds
import * as api from "./namespace_alias_lib.ds";

api.ping();
//^^^ target:namespace_alias
const next = api.ping();
```

```query rename target:namespace_alias "service"
```

```expected:namespace_alias_lib
export function ping(): void {}
```

```expected:namespace_alias_main
import * as service from "./namespace_alias_lib.ds";

service.ping();
const next = service.ping();
```

### Rename default import alias

Renaming a default import alias should update local alias bindings and local call sites only.

```ds:default_alias_lib.ds
export default function greetDefault(name: string): string {
//                        ^^^^^^^^^^^^ def:default_alias_export
    return "Hello, " + name;
}
```

```ds:default_alias_main.ds
import welcome from "./default_alias_lib.ds";

const one = welcome("Destack");
//            ^^^^^^^ target:default_alias
const two = welcome("Team");
```

```query rename target:default_alias "salute"
```

```expected:default_alias_lib
export default function greetDefault(name: string): string {
    return "Hello, " + name;
}
```

```expected:default_alias_main
import salute from "./default_alias_lib.ds";

const one = salute("Destack");
const two = salute("Team");
```

### Rename exported symbol accessed via namespace import

Renaming an exported symbol should update namespace member accesses.

```ds:api.ds
export function greet(name: string): string {
//                ^^^^^ target:ns
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
//               ^ target:typeparam
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
//          ^ target:class-param
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
//                ^ target:interface-param
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
//        ^^^^^ target:destructure

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
//               ^^^^^^^ target:alias

const total = current + 1;
```

```query rename target:alias "amount"
```

```expected:main
const config = { value: 1 };
const { value: amount } = config;

const total = amount + 1;
```

## TypeScript++ Surface

### Rename tagged template tag function

Renaming a tagged template tag should update tagged template call sites.

```ds
function sql(parts: string[], ...values: int32): string {
//         ^^^ target:sql_tag
    return "";
}

const first = sql`select ${1}`;
const second = sql`where ${2}`;
```

```query rename target:sql_tag "query"
```

```expected:main
function query(parts: string[], ...values: int32): string {
    return "";
}

const first = query`select ${1}`;
const second = query`where ${2}`;
```

### Rename decorator function and usages

Renaming a decorator function should update decorator attachment sites.

```ds
function tracked<T>(value: T): T {
//         ^^^^^^^ target:decorator
    return value;
}

@tracked
class Service {}

@tracked
function greet(): void {}
```

```query rename target:decorator "instrumented"
```

```expected:main
function instrumented<T>(value: T): T {
    return value;
}

@instrumented
class Service {}

@instrumented
function greet(): void {}
```

### Rename decorator function on member and parameter attachments

Renaming a decorator function should update member and parameter decorator sites.

```ds
function trackUsage(target: unknown): void {
//         ^^^^^^^^^^ target:decorator_member_param
    target;
}

class User {
    @trackUsage
    name: string = "";
}

function greet(@trackUsage name: string): string {
    return name;
}
```

```query rename target:decorator_member_param "observeUsage"
```

```expected:main
function observeUsage(target: unknown): void {
    target;
}

class User {
    @observeUsage
    name: string = "";
}

function greet(@observeUsage name: string): string {
    return name;
}
```

### Rename match arm binding

Renaming a match arm binding should update in arm references only.

```ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
//     ^^^^ target:match_left
};
```

```query rename target:match_left "first"
```

```expected:main
declare const pair: (int32, int32);

const total = match (pair) {
    (first, right) => first + right
};
```

### Rename template literal type parameter usage

Renaming a type parameter should update template literal type spans.

```ds
type Route<T extends string> = `api:${T}`;
//           ^ target:route_param

type UsersRoute = Route<"users">;
```

```query rename target:route_param "Name"
```

```expected:main
type Route<Name extends string> = `api:${Name}`;

type UsersRoute = Route<"users">;
```

### Rename using binding

Renaming a `using` binding should update local in scope usages.

```ds
using session = 1;
//      ^^^^^^^ target:using_session

const first = session;
const second = session;
```

```query rename target:using_session "resource"
```

```expected:main
using resource = 1;

const first = resource;
const second = resource;
```

### Rename function used in comptime expression

Renaming a function should update call sites in comptime and runtime expressions.

```ds
function build(): int32 {
//         ^^^^^ target:comptime_build
    return 1;
}

const first = comptime build();
const second = build();
```

```query rename target:comptime_build "make"
```

```expected:main
function make(): int32 {
    return 1;
}

const first = comptime make();
const second = make();
```

### Rename local binding inside comptime block

Renaming a local inside a comptime block should update only block local uses.

```ds
const result = comptime {
    let seed = 1;
//        ^^^^ target:comptime_seed
    seed + seed
};
```

```query rename target:comptime_seed "base"
```

```expected:main
const result = comptime {
    let base = 1;
    base + base
};
```

## No-Op Rename

### Rename local binding to the same name

Renaming a local binding to the same name should leave the file unchanged.

```ds
const current = 1;
//      ^^^^^^^ target:same_name
```

```query rename target:same_name "current"
```

```expected:main
const current = 1;
```

## Damaged Syntax

### Rename valid symbols in damaged files

Rename should still work for valid symbols in files with neighboring malformed syntax.

```ds
function main(): void {
    const value = 1;
//          ^^^^^ target:value
    value;
    missingValue.
}
```

```query rename target:value "nextValue"
```

```expected:main
function main(): void {
    const nextValue = 1;
    nextValue;
    missingValue.
}
```

### Rename valid symbols after malformed function declarations

Rename should still work for later declarations after one malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}
//                ^^^^^^^^^^^ target:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query rename target:stableLater "renamedLater"
```

```expected:main
export function broken( {}

export function renamedLater(): void {}

renamedLater();
```

### Rename valid symbols after malformed call statements

Rename should still work for later declarations after one malformed call statement.

```ds
broken(,

export function stableLater(): void {}
//                ^^^^^^^^^^^ target:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query rename target:stableLater "renamedLater"
```

```expected:main
broken(,

export function renamedLater(): void {}

renamedLater();
```

### Rename valid symbols after bare new recovery statements

Rename should still work for later declarations after one bare `new` recovery statement.

```ds
new

export function stableLater(): void {}
//                ^^^^^^^^^^^ target:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query rename target:stableLater "renamedLater"
```

```expected:main
new

export function renamedLater(): void {}

renamedLater();
```

### Rename valid symbols after throw recovery statements

Rename should still work for later declarations after one recovered `throw` statement.

```ds
throw

export function stableLater(): void {}
//                ^^^^^^^^^^^ target:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query rename target:stableLater "renamedLater"
```

```expected:main
throw

export function renamedLater(): void {}

renamedLater();
```

### Rename valid symbols after yield star recovery statements

Rename should still work for later declarations after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

export function stableLater(): void {}
//                ^^^^^^^^^^^ target:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query rename target:stableLater "renamedLater"
```

```expected:main
function* broken() {
    yield*
    const value = 1;
}

export function renamedLater(): void {}

renamedLater();
```

### Reject rename on malformed unresolved member access

Rename should fail when the cursor target is malformed unresolved syntax.

```ds
function main(): void {
    missingValue.
//    ^^^^^^^^^^^^ broken
}
```

```query rename broken "nextValue"
<none>
```

## Associated Types

### Rename associated type projections

Renaming an associated type should update declaration and projection usage sites.

```ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
//         ^^^^^ target:assoc_label
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
```

```query rename target:assoc_label "Kind"
```

```expected:main
interface Envelope<T extends string> {
    type Kind<U extends string> = `${T}:${U}`;
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Kind<"created">;
```

## Using

### Rename await using binding

Renaming an await using binding should update local in-scope references.

```ds
async function run(): void {
    await using resource = 1;
//                ^^^^^^^^ target:await_using_resource

    const next = resource;
}
```

```query rename target:await_using_resource "handle"
```

```expected:main
async function run(): void {
    await using handle = 1;

    const next = handle;
}
```

### Rename for using iteration binding

Renaming a for using iteration binding should update loop-local references.

```ds
declare function values(): int32[];

for (using item of values()) {
//           ^^^^ target:for_using_item
    const next = item;
}
```

```query rename target:for_using_item "value"
```

```expected:main
declare function values(): int32[];

for (using value of values()) {
    const next = value;
}
```

### Rename export using binding

Renaming an export using binding should update declaration and local references.

```ds
export using cache = 1;
//             ^^^^^ target:export_using_cache

const value = cache;
```

```query rename target:export_using_cache "store"
```

```expected:main
export using store = 1;

const value = store;
```

## Comptime

### Rename comptime static parameter

Renaming a comptime static parameter should update indexed type uses.

```ds
type Buffer<comptime N: number> = uint8[N];
//                     ^ target:comptime_n
```

```query rename target:comptime_n "Size"
```

```expected:main
type Buffer<comptime Size: number> = uint8[Size];
```

### Rename comptime dynamic parameter

Renaming a comptime dynamic parameter should update body references.

```ds
function createBuffer(comptime size: int): int {
//                               ^^^^ target:comptime_size
    return size;
}
```

```query rename target:comptime_size "count"
```

```expected:main
function createBuffer(comptime count: int): int {
    return count;
}
```

## Decorators

### Rename match arm decorator

Renaming a decorator should update match arm decorator usage.

```ds
function cold(target: unknown): void {
//         ^^^^ target:arm_decorator
    target;
}

declare const value: int32 | string;

const result = match (value) {
    @cold
    0 => "zero"
    _ => "other"
};
```

```query rename target:arm_decorator "likelyCold"
```

```expected:main
function likelyCold(target: unknown): void {
    target;
}

declare const value: int32 | string;

const result = match (value) {
    @likelyCold
    0 => "zero"
    _ => "other"
};
```

### Rename statement decorator

Renaming a decorator should update statement decorator usage.

```ds
function unroll(target: unknown): void {
//         ^^^^^^ target:statement_decorator
    target;
}

@unroll
for (let i = 0; i < 3; i++) {
    const _ = i;
}
```

```query rename target:statement_decorator "inlineLoop"
```

```expected:main
function inlineLoop(target: unknown): void {
    target;
}

@inlineLoop
for (let i = 0; i < 3; i++) {
    const _ = i;
}
```

## Imports And Exports

### Rename import alias binding

Renaming a Destack import alias should update alias declaration and local references.

```ds:main.ds
namespace bar {
    export const baz = 1;
}

import Foo = bar.baz;
//       ^^^ target:import_alias

const value = Foo;
```

```query rename target:import_alias "BarValue"
```

```expected:main.ds
namespace bar {
    export const baz = 1;
}

import BarValue = bar.baz;

const value = BarValue;
```

### Rename exported symbol through export namespace re export chains

Renaming an exported symbol should update namespace re export call sites.

```ds:base.ds
export function ping(): void {}
//                ^^^^ target:ns_export_target
```

```ds:barrel.ds
export * as api from "./base.ds";
```

```ds:main.ds
import { api } from "./barrel.ds";

api.ping();
```

```query rename target:ns_export_target "pong"
```

```expected:base
export function pong(): void {}
```

```expected:barrel
export * as api from "./base.ds";
```

```expected:main
import { api } from "./barrel.ds";

api.pong();
```

## Ownership And This

### Rename ownership type symbols

Renaming a type should update borrowed, owned, and pointer type usages.

```ds
struct Buffer {
//       ^^^^^^ target:buffer
    value: int32
}

declare const borrowed: &Buffer;
declare const owned: ^Buffer;
declare const pointer: *Buffer;
```

```query rename target:buffer "Memory"
```

```expected:main
struct Memory {
    value: int32
}

declare const borrowed: &Memory;
declare const owned: ^Memory;
declare const pointer: *Memory;
```

### Rename class used in explicit this parameter

Renaming a class should update explicit this parameter type annotations.

```ds
class Counter {}
//      ^^^^^^^ target:counter

function read(this: Counter): Counter {
    return this;
}
```

```query rename target:counter "Meter"
```

```expected:main
class Meter {}

function read(this: Meter): Meter {
    return this;
}
```
