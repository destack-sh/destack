
## Local Symbols

### Find a local declaration and every use

References follow source order, with the declaration included only when requested.

```tspp main.tspp
const value = 1;
      ^^^^^ declaration
const first = value;
              ^^^^^ first_reference
const second = value;
               ^^^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#value@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#value@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#value@1
```

```query find_references main.tspp#first_reference
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#value@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#value@1
```

### Find read and write occurrences

Assignments and reads share one lexical identity.

```tspp main.tspp
let value = 0;
    ^^^^^ declaration

value = 1;
^^^^^ write

const result = value;
               ^^^^^ read
```

```query find_references main.tspp#read include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#value@1
@find_references.reference location=main.tspp#write symbol=main.tspp#value@1
@find_references.reference location=main.tspp#read symbol=main.tspp#value@1
```

### Return current references

References include every current occurrence in source order.

```tspp main.tspp
const value = 1;
      ^^^^^ declaration
const first = value;
              ^^^^^ first
```

```query find_references main.tspp#first include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#value@1
@find_references.reference location=main.tspp#first symbol=main.tspp#value@1
```

```tspp main.tspp change
const value = 1;
      ^^^^^ declaration
const first = value;
              ^^^^^ first
const second = value;
               ^^^^^ second
```

```query find_references main.tspp#first include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#value@1
@find_references.reference location=main.tspp#first symbol=main.tspp#value@1
@find_references.reference location=main.tspp#second symbol=main.tspp#value@1
```

## Modules

### Find references across modules

Cross-module references follow the exported symbol through its import.

```tspp library.tspp
export function greet(name: string): string {
                ^^^^^ declaration
    return name;
}
```

```tspp main.tspp
import { greet } from "./library.tspp";
         ^^^^^ import

const first = greet("one");
              ^^^^^ first_reference
const second = greet("two");
               ^^^^^ second_reference
```

```query find_references library.tspp#declaration include_declaration=true
@find_references.reference location=library.tspp#declaration symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#import symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#first_reference symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#second_reference symbol=library.tspp#greet@1
```

### Find builtin references from user source

Builtin declarations and physical uses share one reference identity.

```tspp main.tspp
import { log } from "tspp:console";
         ^^^ import

log("ready");
^^^ reference
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=tspp://console/console:105:17-105:20 symbol=tspp://console/console#log@61
@find_references.reference location=main.tspp#import symbol=tspp://console/console#log@61
@find_references.reference location=main.tspp#reference symbol=tspp://console/console#log@61
```

## Members

### Find field occurrences

Field references include the declaration and every access.

```tspp main.tspp
struct Point {
    x: int32;
    ^ declaration
}

function read(point: Point): int32 {
    return point.x;
                 ^ reference
}
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#x@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#x@2
```

## Import Aliases

### Find references to a local import alias

An explicit local alias has its own declaration and reference set.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";
                  ^^^^^^^ declaration

welcome();
^^^^^^^ first_reference
welcome();
^^^^^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#welcome@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#welcome@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#welcome@1
```

### Find references from the imported side of an alias

The imported name follows the exported declaration while the explicit alias remains locally renameable.

```tspp library.tspp
export function greet(): void {}
                ^^^^^ declaration
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";
         ^^^^^ imported_name

welcome();
^^^^^^^ reference
```

```query find_references main.tspp#imported_name include_declaration=true
@find_references.reference location=library.tspp#declaration symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#imported_name symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#reference symbol=library.tspp#greet@1
```

## Shadowing

### Find only one shadowed binding

Shadowed names have separate reference identities.

```tspp main.tspp
const value = 1;

function inner(): int32 {
    const value = 2;
          ^^^^^ declaration
    return value;
           ^^^^^ reference
}

const outer = value;
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#value@3
@find_references.reference location=main.tspp#reference symbol=main.tspp#value@3
```

## Methods

### Find method occurrences

Method references include the declaration and every call.

```tspp main.tspp
class Service {
    run(): void {}
    ^^^ declaration
}

function start(service: Service): void {
    service.run();
            ^^^ first_reference
    service.run();
            ^^^ second_reference
}
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#run@2
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#run@2
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#run@2
```

### Find extension method occurrences

Extension calls belong to their extension method.

```tspp main.tspp
struct Calculator {}

extension of Calculator {
    add(left: int32, right: int32): int32 {
    ^^^ declaration
        return left + right;
    }
}

function total(calculator: Calculator): int32 {
    return calculator.add(1, 2) + calculator.add(3, 4);
                      ^^^ first_reference
                                             ^^^ second_reference
}
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#add@3
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#add@3
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#add@3
```

### Find associated constant occurrences

Associated constant accesses belong to their member declaration.

```tspp main.tspp
struct Buffer {
    const Width: uint64 = 8;
          ^^^^^ declaration
}

const first = Buffer.Width;
                     ^^^^^ first_reference
const second = Buffer.Width;
                      ^^^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#Width@2
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#Width@2
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#Width@2
```

### Find every target of a union member access

A union member access belongs to every member declaration it can reach.

```tspp main.tspp
class Alpha {
    run(): void {}
    ^^^ alpha_declaration
}

class Beta {
    run(): void {}
    ^^^ beta_declaration
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ reference
}
```

```query find_references main.tspp#alpha_declaration include_declaration=true
@find_references.reference location=main.tspp#alpha_declaration symbol=main.tspp#run@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#run@2
```

```query find_references main.tspp#beta_declaration include_declaration=true
@find_references.reference location=main.tspp#beta_declaration symbol=main.tspp#run@5
@find_references.reference location=main.tspp#reference symbol=main.tspp#run@5
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#alpha_declaration symbol=main.tspp#run@2
@find_references.reference location=main.tspp#beta_declaration symbol=main.tspp#run@5
@find_references.reference location=main.tspp#reference symbols=main.tspp#run@2,main.tspp#run@5
```

## Enum Members

### Find enum member occurrences

Enum member references belong to their variant declaration.

```tspp main.tspp
enum Color {
    Red,
    ^^^ declaration
}

const first = Color.Red;
                    ^^^ first_reference
const second = Color.Red;
                     ^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#Red@2
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#Red@2
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#Red@2
```

## Nominal Types

### Find type, value, and construction occurrences

A nominal declaration has one identity across type annotations, values, and construction.

```tspp main.tspp
class User {}
      ^^^^ declaration

declare const existing: User;
                        ^^^^ type_reference
const created = new User();
                    ^^^^ construction_reference
```

```query find_references main.tspp#type_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#User@1
@find_references.reference location=main.tspp#type_reference symbol=main.tspp#User@1
@find_references.reference location=main.tspp#construction_reference symbol=main.tspp#User@1
```

## Overloads

### Find only occurrences of one overload

Each overload declaration owns only the calls that match it.

```tspp main.tspp
function parse(value: int32): int32 {
         ^^^^^ integer_declaration
    return value;
}

function parse(value: string): string {
         ^^^^^ string_declaration
    return value;
}

const integerValue = parse(1);
                     ^^^^^ integer_reference
const stringValue = parse("one");
                    ^^^^^ string_reference
```

```query find_references main.tspp#integer_reference include_declaration=true
@find_references.reference location=main.tspp#integer_declaration symbol=main.tspp#parse@1
@find_references.reference location=main.tspp#integer_reference symbol=main.tspp#parse@1
```

```query find_references main.tspp#string_reference include_declaration=true
@find_references.reference location=main.tspp#string_declaration symbol=main.tspp#parse@3
@find_references.reference location=main.tspp#string_reference symbol=main.tspp#parse@3
```

## Generic Parameters

### Find type parameter occurrences

A generic parameter includes its declaration and every type-position reference.

```tspp main.tspp
function identity<T>(value: T): T {
                  ^ declaration
                            ^ first_reference
                                ^ second_reference
    return value;
}
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#T@2
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#T@2
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#T@2
```

### Find a type parameter inside a template literal type

Template literal types include their generic parameter references.

```tspp main.tspp
type Route<T: string> = `api:${T}`;
           ^ declaration
                               ^ reference
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#T@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#T@2
```

### Find a const function parameter

A const parameter has one lexical identity inside the function body.

```tspp main.tspp
function createBuffer<const size: int32>(): int32 {
                            ^^^^ declaration
    return size;
           ^^^^ reference
}
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#size@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#size@2
```

### Find a const type parameter

A const parameter has one lexical identity inside the declared type.

```tspp main.tspp
type Buffer<const size: usize> = [uint8; size];
                  ^^^^ declaration
                                         ^^^^ reference
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#size@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#size@2
```

## Pattern Bindings

### Find destructured binding occurrences

A destructured binding has its own lexical reference set.

```tspp main.tspp
const pair = { left: 1, right: 2 };
const { left } = pair;
        ^^^^ declaration

const value = left;
              ^^^^ reference
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#left@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#left@2
```

### Find match binding occurrences

A match-arm binding includes only references inside its arm.

```tspp main.tspp
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ declaration
                     ^^^^ reference
};
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#left@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#left@2
```

## Labels

### Find control label occurrences

A control label includes its declaration and each targeted break.

```tspp main.tspp
function choose(value: boolean): int32 {
    outer: loop {
    ^^^^^ declaration
        if (value) {
            break outer: 1;
                  ^^^^^ first_reference
        }

        break outer: 2;
              ^^^^^ second_reference
    }
}
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#outer@3
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#outer@3
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#outer@3
```

## Calls

### Find tagged-template function occurrences

A tag function includes ordinary and tagged-template calls.

```tspp main.tspp
function sql(parts: string[], ...values: int32[]): string {
         ^^^ declaration
    return "";
}

const first = sql`select ${1}`;
              ^^^ first_reference
const second = sql(["select"], 2);
               ^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#sql@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#sql@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#sql@1
```

### Find const call occurrences

Const and runtime calls share the same function identity.

```tspp main.tspp
function scale(value: int32): int32 {
         ^^^^^ declaration
    return value * 2;
}

const first = const scale(2);
                    ^^^^^ first_reference
const second = scale(3);
               ^^^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#scale@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#scale@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#scale@1
```

### Do not attribute an indirect call to the assigned function

Assigning a function creates a reference, but calling the binding does not create another reference to it.

```tspp main.tspp
function callee(): void {}
         ^^^^^^ declaration

const callback = callee;
                 ^^^^^^ reference
callback();
```

```query find_references main.tspp#declaration include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#callee@1
@find_references.reference location=main.tspp#reference symbol=main.tspp#callee@1
```

## Decorators

### Find decorator occurrences

A decorator includes its declaration and every application.

```tspp main.tspp
newtype tracked = ();
        ^^^^^^^ declaration

@tracked
 ^^^^^^^ first_reference
class Service {}

@tracked
 ^^^^^^^ second_reference
function start(): void {}
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#tracked@1
```

### Find decorator occurrences across attachment positions

A decorator has one identity across every supported attachment position.

```tspp main.tspp
newtype tracked = ();
        ^^^^^^^ declaration

class Service {
    @tracked
     ^^^^^^^ member_reference
    value: int32;
}

function start(@tracked value: int32): void {
                ^^^^^^^ parameter_reference
}

declare const input: int32;

const result = match (input) {
    @tracked
     ^^^^^^^ arm_reference
    0 => 0
    _ => 1
};

switch (input) {
    @tracked
     ^^^^^^^ case_reference
    case 0: break;
    default: break;
}

@tracked
 ^^^^^^^ statement_reference
for (let index = 0; index < 1; index++) {}
```

```query find_references main.tspp#member_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#member_reference symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#parameter_reference symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#arm_reference symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#case_reference symbol=main.tspp#tracked@1
@find_references.reference location=main.tspp#statement_reference symbol=main.tspp#tracked@1
```

## Using Bindings

### Find using binding occurrences

A using binding participates in ordinary lexical reference lookup.

```tspp main.tspp
declare function openSession(): Dispose;

using session = openSession();
      ^^^^^^^ declaration

const first = session;
              ^^^^^^^ first_reference
const second = session;
               ^^^^^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#session@2
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#session@2
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#session@2
```

### Find an await-using binding

An await-using declaration introduces an async lexical resource binding.

```tspp main.tspp
declare function openResource(): AsyncDispose;

async function run(): void {
    await using resource = openResource();
                ^^^^^^^^ declaration

    const value = resource;
                  ^^^^^^^^ reference
}
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#resource@3
@find_references.reference location=main.tspp#reference symbol=main.tspp#resource@3
```

### Find a for-using binding

A for-using declaration has one lexical identity inside the loop body.

```tspp main.tspp
declare function values(): Dispose[];

for (using item of values()) {
           ^^^^ declaration
    const next = item;
                 ^^^^ reference
}
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#item@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#item@2
```

### Find an exported using binding

An exported using declaration keeps its local lexical identity.

```tspp main.tspp
declare function openCache(): Dispose;

export using cache = openCache();
             ^^^^^ declaration

const value = cache;
              ^^^^^ reference
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#cache@2
@find_references.reference location=main.tspp#reference symbol=main.tspp#cache@2
```

## Import Forms

### Find references to an imported type

A type includes its plain import and annotation occurrences.

```tspp library.tspp
export type Options = {
            ^^^^^^^ declaration
    enabled: boolean,
};
```

```tspp main.tspp
import { Options } from "./library.tspp";
         ^^^^^^^ import

declare const first: Options;
                     ^^^^^^^ first_reference
declare const second: Options;
                      ^^^^^^^ second_reference
```

```query find_references library.tspp#declaration include_declaration=true
@find_references.reference location=library.tspp#declaration symbol=library.tspp#Options@1
@find_references.reference location=main.tspp#import symbol=library.tspp#Options@1
@find_references.reference location=main.tspp#first_reference symbol=library.tspp#Options@1
@find_references.reference location=main.tspp#second_reference symbol=library.tspp#Options@1
```

### Find namespace import alias occurrences

A namespace import alias has a local declaration and receiver references.

```tspp library.tspp
export function ping(): void {}
```

```tspp main.tspp
import * as api from "./library.tspp";
            ^^^ declaration

api.ping();
^^^ first_reference
(api).ping();
 ^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#api@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#api@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#api@1
```

### Find a namespace segment in a qualified type reference

A namespace segment written in a type path references its import declaration.

```tspp library.tspp
export type Handler = () => void;
export function ping(): void {}
```

```tspp main.tspp
import * as api from "./library.tspp";
            ^^^ declaration

const handler: api.Handler = api.ping;
               ^^^ type_reference
                             ^^^ value_reference
```

```query find_references main.tspp#declaration include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#api@1
@find_references.reference location=main.tspp#type_reference symbol=main.tspp#api@1
@find_references.reference location=main.tspp#value_reference symbol=main.tspp#api@1
```

### Find default import binding occurrences

A default import binding remains a local alias in the importing module.

```tspp library.tspp
export default function greet(): void {}
```

```tspp main.tspp
import welcome from "./library.tspp";
       ^^^^^^^ declaration

welcome();
^^^^^^^ first_reference
welcome();
^^^^^^^ second_reference
```

```query find_references main.tspp#first_reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#welcome@1
@find_references.reference location=main.tspp#first_reference symbol=main.tspp#welcome@1
@find_references.reference location=main.tspp#second_reference symbol=main.tspp#welcome@1
```

### Find references through a default re-export

References include each re-export, import, and call occurrence.

```tspp library.tspp
export default function buildWidget(): int32 {
                        ^^^^^^^^^^^ declaration
    return 1;
}
```

```tspp public.tspp
export { default as buildWidget } from "./library.tspp";
                    ^^^^^^^^^^^ reexport
```

```tspp main.tspp
import { buildWidget } from "./public.tspp";
         ^^^^^^^^^^^ import

const value = buildWidget();
              ^^^^^^^^^^^ reference
```

```query find_references library.tspp#declaration include_declaration=true
@find_references.reference location=library.tspp#declaration symbol=library.tspp#buildWidget@1
@find_references.reference location=public.tspp#reexport symbol=library.tspp#buildWidget@1
@find_references.reference location=main.tspp#import symbol=library.tspp#buildWidget@1
@find_references.reference location=main.tspp#reference symbol=library.tspp#buildWidget@1
```

### Find references through a named re-export

A named export preserves the original symbol through the re-export, import, and call.

```tspp library.tspp
export function greet(): void {}
                ^^^^^ declaration
```

```tspp barrel.tspp
export { greet } from "./library.tspp";
         ^^^^^ reexport
```

```tspp main.tspp
import { greet } from "./barrel.tspp";
         ^^^^^ import

greet();
^^^^^ reference
```

```query find_references library.tspp#declaration include_declaration=true
@find_references.reference location=library.tspp#declaration symbol=library.tspp#greet@1
@find_references.reference location=barrel.tspp#reexport symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#import symbol=library.tspp#greet@1
@find_references.reference location=main.tspp#reference symbol=library.tspp#greet@1
```

### Find a namespace re-export alias

A namespace re-export alias has one identity through imports and local uses.

```tspp base.tspp
export function ping(): void {}
```

```tspp barrel.tspp
export * as api from "./base.tspp";
            ^^^ declaration
```

```tspp main.tspp
import { api } from "./barrel.tspp";
         ^^^ import

api.ping();
^^^ first_reference

const same = api;
             ^^^ second_reference
```

```query find_references main.tspp#import include_declaration=true
@find_references.reference location=barrel.tspp#declaration symbol=barrel.tspp#api@1
@find_references.reference location=main.tspp#import symbol=barrel.tspp#api@1
@find_references.reference location=main.tspp#first_reference symbol=barrel.tspp#api@1
@find_references.reference location=main.tspp#second_reference symbol=barrel.tspp#api@1
```

### Find a nested namespace in type paths

A namespace re-export has the same reference identity in each qualified type path.

```tspp model.tspp
export struct Packet {}
```

```tspp library.tspp
export * as models from "./model";
            ^^^^^^ declaration
```

```tspp main.tspp
import * as library from "./library";

declare const first: library.models.Packet;
                             ^^^^^^ first

declare const second: library.models.Packet;
                              ^^^^^^ second
```

```query find_references main.tspp#first include_declaration=true
@find_references.reference location=library.tspp#declaration symbol=library.tspp#models@1
@find_references.reference location=main.tspp#first symbol=library.tspp#models@1
@find_references.reference location=main.tspp#second symbol=library.tspp#models@1
```

## Associated Types

### Find associated type occurrences

An associated declaration includes each type projection that names it.

```tspp main.tspp
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
         ^^^^^ declaration
}

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference
```

```query find_references main.tspp#reference include_declaration=true
@find_references.reference location=main.tspp#declaration symbol=main.tspp#Label@3
@find_references.reference location=main.tspp#reference symbol=main.tspp#Label@3
```

## Missing Symbols

### Return no references for an unresolved name

An unresolved occurrence has no reference identity.

```tspp main.tspp
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query find_references main.tspp#reference include_declaration=true
@find_references.none
```
