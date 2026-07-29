
## Local Symbols

### Find a local declaration and every use

References follow source order, with the declaration included only when requested.

```ds main.ds
const value = 1;
      ^^^^^ declaration
const first = value;
              ^^^^^ first_reference
const second = value;
               ^^^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#value@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#value@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#value@1
```

```query find_references main.ds#first_reference
@find_references.reference location=main.ds#first_reference symbol=main.ds#value@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#value@1
```

### Find read and write occurrences

Assignments and reads share one lexical identity.

```ds main.ds
let value = 0;
    ^^^^^ declaration

value = 1;
^^^^^ write

const result = value;
               ^^^^^ read
```

```query find_references main.ds#read include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#value@1
@find_references.reference location=main.ds#write symbol=main.ds#value@1
@find_references.reference location=main.ds#read symbol=main.ds#value@1
```

## Modules

### Find references across modules

Cross-module references follow the exported symbol through its import.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ declaration
    return name;
}
```

```ds main.ds
import { greet } from "./library.ds";
         ^^^^^ import

const first = greet("one");
              ^^^^^ first_reference
const second = greet("two");
               ^^^^^ second_reference
```

```query find_references library.ds#declaration include_declaration=true
@find_references.reference location=library.ds#declaration symbol=library.ds#greet@1
@find_references.reference location=main.ds#import symbol=library.ds#greet@1
@find_references.reference location=main.ds#first_reference symbol=library.ds#greet@1
@find_references.reference location=main.ds#second_reference symbol=library.ds#greet@1
```

## Members

### Find field occurrences

Field references include the declaration and every access.

```ds main.ds
struct Point {
    x: int32;
    ^ declaration
}

function read(point: Point): int32 {
    return point.x;
                 ^ reference
}
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#x@2
@find_references.reference location=main.ds#reference symbol=main.ds#x@2
```

## Import Aliases

### Find references to a local import alias

An explicit local alias has its own declaration and reference set.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { greet as welcome } from "./library.ds";
                  ^^^^^^^ declaration

welcome();
^^^^^^^ first_reference
welcome();
^^^^^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#welcome@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#welcome@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#welcome@1
```

### Find references from the imported side of an alias

The imported name follows the exported declaration while the explicit alias remains locally renameable.

```ds library.ds
export function greet(): void {}
                ^^^^^ declaration
```

```ds main.ds
import { greet as welcome } from "./library.ds";
         ^^^^^ imported_name

welcome();
^^^^^^^ reference
```

```query find_references main.ds#imported_name include_declaration=true
@find_references.reference location=library.ds#declaration symbol=library.ds#greet@1
@find_references.reference location=main.ds#imported_name symbol=library.ds#greet@1
@find_references.reference location=main.ds#reference symbol=library.ds#greet@1
```

## Shadowing

### Find only one shadowed binding

Shadowed names have separate reference identities.

```ds main.ds
const value = 1;

function inner(): int32 {
    const value = 2;
          ^^^^^ declaration
    return value;
           ^^^^^ reference
}

const outer = value;
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#value@3
@find_references.reference location=main.ds#reference symbol=main.ds#value@3
```

## Methods

### Find method occurrences

Method references include the declaration and every call.

```ds main.ds
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

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#run@2
@find_references.reference location=main.ds#first_reference symbol=main.ds#run@2
@find_references.reference location=main.ds#second_reference symbol=main.ds#run@2
```

### Find extension method occurrences

Extension calls belong to their extension method.

```ds main.ds
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

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#add@3
@find_references.reference location=main.ds#first_reference symbol=main.ds#add@3
@find_references.reference location=main.ds#second_reference symbol=main.ds#add@3
```

### Find associated constant occurrences

Associated constant accesses belong to their member declaration.

```ds main.ds
struct Buffer {
    comptime const Width: uint64 = 8;
                   ^^^^^ declaration
}

const first = Buffer.Width;
                     ^^^^^ first_reference
const second = Buffer.Width;
                      ^^^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#Width@2
@find_references.reference location=main.ds#first_reference symbol=main.ds#Width@2
@find_references.reference location=main.ds#second_reference symbol=main.ds#Width@2
```

### Find every target of a union member access

A union member access belongs to every member declaration it can reach.

```ds main.ds
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

```query find_references main.ds#alpha_declaration include_declaration=true
@find_references.reference location=main.ds#alpha_declaration symbol=main.ds#run@2
@find_references.reference location=main.ds#reference symbol=main.ds#run@2
```

```query find_references main.ds#beta_declaration include_declaration=true
@find_references.reference location=main.ds#beta_declaration symbol=main.ds#run@5
@find_references.reference location=main.ds#reference symbol=main.ds#run@5
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#alpha_declaration symbol=main.ds#run@2
@find_references.reference location=main.ds#beta_declaration symbol=main.ds#run@5
@find_references.reference location=main.ds#reference symbols=main.ds#run@2,main.ds#run@5
```

## Enum Members

### Find enum member occurrences

Enum member references belong to their variant declaration.

```ds main.ds
enum Color {
    Red,
    ^^^ declaration
}

const first = Color.Red;
                    ^^^ first_reference
const second = Color.Red;
                     ^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#Red@2
@find_references.reference location=main.ds#first_reference symbol=main.ds#Red@2
@find_references.reference location=main.ds#second_reference symbol=main.ds#Red@2
```

### [ignored] Find tagged variant occurrences

Tagged construction and patterns belong to their variant declaration.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string> | Error<int32>;
                 ^^ declaration

const status = Status.Ok({ value: "ready" });
                      ^^ construction_reference

const message = match (status) {
    Status.Ok(value) => value
           ^^ pattern_reference
    Status.Error(code) => ""
};
```

```query find_references main.ds#construction_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#Ok@3
@find_references.reference location=main.ds#construction_reference symbol=main.ds#Ok@3
@find_references.reference location=main.ds#pattern_reference symbol=main.ds#Ok@3
```

## Nominal Types

### Find type, value, and construction occurrences

A nominal declaration has one identity across type annotations, values, and construction.

```ds main.ds
class User {}
      ^^^^ declaration

declare const existing: User;
                        ^^^^ type_reference
const created = new User();
                    ^^^^ construction_reference
```

```query find_references main.ds#type_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#User@1
@find_references.reference location=main.ds#type_reference symbol=main.ds#User@1
@find_references.reference location=main.ds#construction_reference symbol=main.ds#User@1
```

## Overloads

### Find only occurrences of one overload

Each overload declaration owns only the calls that match it.

```ds main.ds
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

```query find_references main.ds#integer_reference include_declaration=true
@find_references.reference location=main.ds#integer_declaration symbol=main.ds#parse@1
@find_references.reference location=main.ds#integer_reference symbol=main.ds#parse@1
```

```query find_references main.ds#string_reference include_declaration=true
@find_references.reference location=main.ds#string_declaration symbol=main.ds#parse@3
@find_references.reference location=main.ds#string_reference symbol=main.ds#parse@3
```

## Generic Parameters

### Find type parameter occurrences

A generic parameter includes its declaration and every type-position reference.

```ds main.ds
function identity<T>(value: T): T {
                  ^ declaration
                            ^ first_reference
                                ^ second_reference
    return value;
}
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#T@2
@find_references.reference location=main.ds#first_reference symbol=main.ds#T@2
@find_references.reference location=main.ds#second_reference symbol=main.ds#T@2
```

### Find a type parameter inside a template literal type

Template literal types include their generic parameter references.

```ds main.ds
type Route<T extends string> = `api:${T}`;
           ^ declaration
                                      ^ reference
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#T@2
@find_references.reference location=main.ds#reference symbol=main.ds#T@2
```

### Find a comptime function parameter

A comptime parameter has one lexical identity inside the function body.

```ds main.ds
function createBuffer<comptime size: int32>(): int32 {
                               ^^^^ declaration
    return size;
           ^^^^ reference
}
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#size@2
@find_references.reference location=main.ds#reference symbol=main.ds#size@2
```

### [ignored] Find a comptime type parameter

A comptime parameter has one lexical identity inside the declared type.

```ds main.ds
type Buffer<comptime size: usize> = [uint8; size];
                     ^^^^ declaration
                                           ^^^^ reference
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#size@2
@find_references.reference location=main.ds#reference symbol=main.ds#size@2
```

## Pattern Bindings

### Find destructured binding occurrences

A destructured binding has its own lexical reference set.

```ds main.ds
const pair = { left: 1, right: 2 };
const { left } = pair;
        ^^^^ declaration

const value = left;
              ^^^^ reference
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#left@2
@find_references.reference location=main.ds#reference symbol=main.ds#left@2
```

### [ignored] Find match binding occurrences

A match-arm binding includes only references inside its arm.

```ds main.ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ declaration
                    ^^^^ reference
};
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#left@3
@find_references.reference location=main.ds#reference symbol=main.ds#left@3
```

## Labels

### [ignored] Find control label occurrences

A control label includes its declaration and each targeted break.

```ds main.ds
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

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#outer@3
@find_references.reference location=main.ds#first_reference symbol=main.ds#outer@3
@find_references.reference location=main.ds#second_reference symbol=main.ds#outer@3
```

## Calls

### Find tagged-template function occurrences

A tag function includes ordinary and tagged-template calls.

```ds main.ds
function sql(parts: string[], ...values: int32): string {
         ^^^ declaration
    return "";
}

const first = sql`select ${1}`;
              ^^^ first_reference
const second = sql(["select"], 2);
               ^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#sql@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#sql@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#sql@1
```

### Find comptime call occurrences

Comptime and runtime calls share the same function identity.

```ds main.ds
function scale(value: int32): int32 {
         ^^^^^ declaration
    return value * 2;
}

const first = comptime scale(2);
                       ^^^^^ first_reference
const second = scale(3);
               ^^^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#scale@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#scale@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#scale@1
```

### Do not attribute an indirect call to the assigned function

Assigning a function creates a reference, but calling the binding does not create another reference to it.

```ds main.ds
function callee(): void {}
         ^^^^^^ declaration

const callback = callee;
                 ^^^^^^ reference
callback();
```

```query find_references main.ds#declaration include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#callee@1
@find_references.reference location=main.ds#reference symbol=main.ds#callee@1
```

## Decorators

### Find decorator occurrences

A decorator includes its declaration and every application.

```ds main.ds
newtype tracked = ();
        ^^^^^^^ declaration

@tracked
 ^^^^^^^ first_reference
class Service {}

@tracked
 ^^^^^^^ second_reference
function start(): void {}
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#tracked@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#tracked@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#tracked@1
```

### Find decorator occurrences across attachment positions

A decorator has one identity across every supported attachment position.

```ds main.ds
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

@tracked
 ^^^^^^^ statement_reference
for (let index = 0; index < 1; index++) {}
```

```query find_references main.ds#member_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#tracked@1
@find_references.reference location=main.ds#member_reference symbol=main.ds#tracked@1
@find_references.reference location=main.ds#parameter_reference symbol=main.ds#tracked@1
@find_references.reference location=main.ds#arm_reference symbol=main.ds#tracked@1
@find_references.reference location=main.ds#statement_reference symbol=main.ds#tracked@1
```

## Using Bindings

### Find using binding occurrences

A using binding participates in ordinary lexical reference lookup.

```ds main.ds
declare function openSession(): Dispose;

using session = openSession();
      ^^^^^^^ declaration

const first = session;
              ^^^^^^^ first_reference
const second = session;
               ^^^^^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#session@2
@find_references.reference location=main.ds#first_reference symbol=main.ds#session@2
@find_references.reference location=main.ds#second_reference symbol=main.ds#session@2
```

### Find an await-using binding

An await-using declaration introduces an async lexical resource binding.

```ds main.ds
declare function openResource(): AsyncDispose;

async function run(): void {
    await using resource = openResource();
                ^^^^^^^^ declaration

    const value = resource;
                  ^^^^^^^^ reference
}
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#resource@3
@find_references.reference location=main.ds#reference symbol=main.ds#resource@3
```

### Find a for-using binding

A for-using declaration has one lexical identity inside the loop body.

```ds main.ds
declare function values(): Dispose[];

for (using item of values()) {
           ^^^^ declaration
    const next = item;
                 ^^^^ reference
}
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#item@2
@find_references.reference location=main.ds#reference symbol=main.ds#item@2
```

### Find an exported using binding

An exported using declaration keeps its local lexical identity.

```ds main.ds
declare function openCache(): Dispose;

export using cache = openCache();
             ^^^^^ declaration

const value = cache;
              ^^^^^ reference
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#cache@2
@find_references.reference location=main.ds#reference symbol=main.ds#cache@2
```

## Import Forms

### Find references to an imported type

A type includes its plain import and annotation occurrences.

```ds library.ds
export type Options = {
            ^^^^^^^ declaration
    enabled: boolean,
};
```

```ds main.ds
import { Options } from "./library.ds";
         ^^^^^^^ import

declare const first: Options;
                     ^^^^^^^ first_reference
declare const second: Options;
                      ^^^^^^^ second_reference
```

```query find_references library.ds#declaration include_declaration=true
@find_references.reference location=library.ds#declaration symbol=library.ds#Options@1
@find_references.reference location=main.ds#import symbol=library.ds#Options@1
@find_references.reference location=main.ds#first_reference symbol=library.ds#Options@1
@find_references.reference location=main.ds#second_reference symbol=library.ds#Options@1
```

### Find namespace import alias occurrences

A namespace import alias has a local declaration and receiver references.

```ds library.ds
export function ping(): void {}
```

```ds main.ds
import * as api from "./library.ds";
            ^^^ declaration

api.ping();
^^^ first_reference
(api).ping();
 ^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#api@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#api@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#api@1
```

### Find default import binding occurrences

A default import binding remains a local alias in the importing module.

```ds library.ds
export default function greet(): void {}
```

```ds main.ds
import welcome from "./library.ds";
       ^^^^^^^ declaration

welcome();
^^^^^^^ first_reference
welcome();
^^^^^^^ second_reference
```

```query find_references main.ds#first_reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#welcome@1
@find_references.reference location=main.ds#first_reference symbol=main.ds#welcome@1
@find_references.reference location=main.ds#second_reference symbol=main.ds#welcome@1
```

### Find references through a default re-export

References include each re-export, import, and call occurrence.

```ds library.ds
export default function buildWidget(): int32 {
                        ^^^^^^^^^^^ declaration
    return 1;
}
```

```ds public.ds
export { default as buildWidget } from "./library.ds";
                    ^^^^^^^^^^^ reexport
```

```ds main.ds
import { buildWidget } from "./public.ds";
         ^^^^^^^^^^^ import

const value = buildWidget();
              ^^^^^^^^^^^ reference
```

```query find_references library.ds#declaration include_declaration=true
@find_references.reference location=library.ds#declaration symbol=library.ds#buildWidget@1
@find_references.reference location=public.ds#reexport symbol=library.ds#buildWidget@1
@find_references.reference location=main.ds#import symbol=library.ds#buildWidget@1
@find_references.reference location=main.ds#reference symbol=library.ds#buildWidget@1
```

### Find references through a named re-export

A named export preserves the original symbol through the re-export, import, and call.

```ds library.ds
export function greet(): void {}
                ^^^^^ declaration
```

```ds barrel.ds
export { greet } from "./library.ds";
         ^^^^^ reexport
```

```ds main.ds
import { greet } from "./barrel.ds";
         ^^^^^ import

greet();
^^^^^ reference
```

```query find_references library.ds#declaration include_declaration=true
@find_references.reference location=library.ds#declaration symbol=library.ds#greet@1
@find_references.reference location=barrel.ds#reexport symbol=library.ds#greet@1
@find_references.reference location=main.ds#import symbol=library.ds#greet@1
@find_references.reference location=main.ds#reference symbol=library.ds#greet@1
```

### [ignored] Find a namespace re-export alias

A namespace re-export alias has one identity through imports and local uses.

```ds base.ds
export function ping(): void {}
```

```ds barrel.ds
export * as api from "./base.ds";
            ^^^ declaration
```

```ds main.ds
import { api } from "./barrel.ds";
         ^^^ import

api.ping();
^^^ first_reference

const same = api;
             ^^^ second_reference
```

```query find_references main.ds#import include_declaration=true
@find_references.reference location=barrel.ds#declaration symbol=barrel.ds#api@1
@find_references.reference location=main.ds#import symbol=barrel.ds#api@1
@find_references.reference location=main.ds#first_reference symbol=barrel.ds#api@1
@find_references.reference location=main.ds#second_reference symbol=barrel.ds#api@1
```

## Associated Types

### [ignored] Find associated type occurrences

An associated declaration includes each type projection that names it.

```ds main.ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
         ^^^^^ declaration
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference
```

```query find_references main.ds#reference include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#Label@3
@find_references.reference location=main.ds#reference symbol=main.ds#Label@3
```

## Missing Symbols

### Return no references for an unresolved name

An unresolved occurrence has no reference identity.

```ds main.ds
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query find_references main.ds#reference include_declaration=true
@find_references.none
```

## Source changes

### Update references after a use is added

References include every current occurrence in source order.

```ds main.ds
const value = 1;
      ^^^^^ declaration
const first = value;
              ^^^^^ first
```

```query find_references main.ds#first include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#value@1
@find_references.reference location=main.ds#first symbol=main.ds#value@1
```

```ds main.ds change
const value = 1;
      ^^^^^ declaration
const first = value;
              ^^^^^ first
const second = value;
               ^^^^^ second
```

```query find_references main.ds#first include_declaration=true
@find_references.reference location=main.ds#declaration symbol=main.ds#value@1
@find_references.reference location=main.ds#first symbol=main.ds#value@1
@find_references.reference location=main.ds#second symbol=main.ds#value@1
```
