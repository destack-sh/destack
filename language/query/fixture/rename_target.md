
## Local Bindings

### Resolve a definition

A definition identifies its symbol and placeholder name.

```tspp main.tspp
const count = 1;
      ^^^^^ definition
const next = count;
```

```query rename_target main.tspp#definition
@rename_target.target placeholder=count location=main.tspp#definition symbol=main.tspp#count@1
```

### Resolve a reference

A reference reports its name range and symbol.

```tspp main.tspp
const count = 1;
const next = count;
             ^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=count location=main.tspp#reference symbol=main.tspp#count@1
```

### Resolve the current binding

The rename target is the symbol selected after each edit.

```tspp main.tspp
const first = 1;
const second = 2;
const selected = first;
                 ^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=first location=main.tspp#reference symbol=main.tspp#first@1
```

```tspp main.tspp change
const first = 1;
const second = 2;
const selected = second;
                 ^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=second location=main.tspp#reference symbol=main.tspp#second@2
```

## Members

### Resolve a field access

A named field access resolves as a rename target.

```tspp main.tspp
struct Counter {
    count: int32;
}

function read(counter: Counter): int32 {
    return counter.count;
                   ^^^^^ reference
}
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=count location=main.tspp#reference symbol=main.tspp#count@2
```

### Resolve a string-keyed field access

A string-keyed field access exposes only the identifier contents as its rename range.

```tspp main.tspp
struct Counter {
    count: int32;
}

function read(counter: Counter): int32 {
    return counter["count"];
                    ^^^^^ reference
}
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=count location=main.tspp#reference symbol=main.tspp#count@2
```

## Methods

### Resolve a method access

A method name is a rename target.

```tspp main.tspp
class Service {
    run(): void {}
}

function start(service: Service): void {
    service.run();
            ^^^ reference
}
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=run location=main.tspp#reference symbol=main.tspp#run@2
```

### Resolve an extension method access

An extension call identifies its extension method.

```tspp main.tspp
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

```query rename_target main.tspp#reference
@rename_target.target placeholder=add location=main.tspp#reference symbol=main.tspp#add@3
```

## Associated Constants

### Resolve an associated constant access

An associated constant identifies its member declaration.

```tspp main.tspp
struct Buffer {
    const Width: uint = 8;
}

const width = Buffer.Width;
                     ^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=Width location=main.tspp#reference symbol=main.tspp#Width@2
```

## Enum Members

### Resolve an enum member access

An enum member occurrence identifies its member declaration.

```tspp main.tspp
enum Color {
    Red,
}

const color = Color.Red;
                    ^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=Red location=main.tspp#reference symbol=main.tspp#Red@2
```

## Member Declarations

### Resolve a field definition

A member name is a rename target.

```tspp main.tspp
struct Point {
    x: int32;
    ^ definition
}
```

```query rename_target main.tspp#definition
@rename_target.target placeholder=x location=main.tspp#definition symbol=main.tspp#x@2
```

## Generic Parameters

### Resolve a type parameter reference

A generic type reference identifies its local parameter.

```tspp main.tspp
function identity<T>(value: T): T {
                                ^ reference
    return value;
}
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=T location=main.tspp#reference symbol=main.tspp#T@2
```

### Resolve a type parameter in a template literal

A type parameter inside a template literal identifies its local parameter.

```tspp main.tspp
type Route<T: string> = `api:${T}`;
                               ^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=T location=main.tspp#reference symbol=main.tspp#T@2
```

### Resolve a const type parameter

A const type parameter identifies its local parameter.

```tspp main.tspp
type Buffer<const size: usize> = [uint8; size];
                                         ^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=size location=main.tspp#reference symbol=main.tspp#size@2
```

## Pattern Bindings

### Resolve a destructured binding

A destructured name is an ordinary lexical rename target.

```tspp main.tspp
const pair = { left: 1, right: 2 };
const { left } = pair;
        ^^^^ definition

const value = left;
              ^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=left location=main.tspp#reference symbol=main.tspp#left@2
```

### Resolve a match binding

A match-arm name identifies its arm-local binding.

```tspp main.tspp
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ definition
                     ^^^^ reference
};
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=left location=main.tspp#reference symbol=main.tspp#left@2
```

### Resolve the local value of an object shorthand

An object shorthand token selects its local value binding for rename preparation.

```tspp main.tspp
const horizontal = 1;

const point = { horizontal };
                ^^^^^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=horizontal location=main.tspp#reference symbol=main.tspp#horizontal@1
```

## Overloads

### Resolve an overload family at a call

Rename preparation identifies every declaration that shares the called function name.

```tspp main.tspp
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const value = parse(1);
              ^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=parse location=main.tspp#reference symbols=main.tspp#parse@1,main.tspp#parse@3
```

## Construction

### Resolve a class construction name

The class name in a construction expression is a rename target for the class identity.

```tspp main.tspp
class User {
    constructor(name: string) {}
}

const user = new User("Ada");
                 ^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=User location=main.tspp#reference symbol=main.tspp#User@1
```

## Labels

### Resolve a control label reference

A targeted break identifies its enclosing label.

```tspp main.tspp
function choose(): int32 {
    outer: loop {
        break outer: 1;
              ^^^^^ reference
    }
}
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=outer location=main.tspp#reference symbol=main.tspp#outer@2
```

## Calls

### Resolve a tagged-template function

A tagged-template tag is the same rename target as an ordinary call.

```tspp main.tspp
function sql(parts: string[], ...values: int32[]): string {
    return "";
}

const query = sql`select ${1}`;
              ^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=sql location=main.tspp#reference symbol=main.tspp#sql@1
```

### Resolve a const call

A const call identifies the called function.

```tspp main.tspp
function build(): int32 {
    return 1;
}

const value = const build();
                    ^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=build location=main.tspp#reference symbol=main.tspp#build@1
```

## Annotations

### Resolve an annotation application

An annotation value is a rename target at its application.

```tspp main.tspp
newtype tracked = ();

@tracked
 ^^^^^^^ reference
class Service {}
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=tracked location=main.tspp#reference symbol=main.tspp#tracked@1
```

### Resolve a parameter annotation

An annotation on a parameter identifies the annotation value.

```tspp main.tspp
newtype tracked = ();

function start(@tracked value: int32): void {}
                ^^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=tracked location=main.tspp#reference symbol=main.tspp#tracked@1
```

## Using Bindings

### Resolve a using binding

A using declaration introduces an ordinary lexical rename target.

```tspp main.tspp
declare function openSession(): Dispose;

using session = openSession();

const value = session;
              ^^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=session location=main.tspp#reference symbol=main.tspp#session@2
```

## Import Bindings

### Resolve a local import alias

An explicit import alias remains a local rename target.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";

welcome();
^^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=welcome location=main.tspp#reference symbol=main.tspp#welcome@1
```

### Resolve a namespace import alias

A namespace receiver identifies its local import alias.

```tspp library.tspp
export function ping(): void {}
```

```tspp main.tspp
import * as api from "./library.tspp";

api.ping();
^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=api location=main.tspp#reference symbol=main.tspp#api@1
```

### Resolve a default import binding

A default import name is local to the importing module.

```tspp library.tspp
export default function greet(): void {}
```

```tspp main.tspp
import welcome from "./library.tspp";

welcome();
^^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=welcome location=main.tspp#reference symbol=main.tspp#welcome@1
```

### Resolve an imported type alias

An explicit local alias remains a rename target in the declaration's type symbol space.

```tspp library.tspp
export type Options = {
    enabled: boolean,
};
```

```tspp main.tspp
import { Options as Settings } from "./library.tspp";

declare const settings: Settings;
                        ^^^^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=Settings location=main.tspp#reference symbol=main.tspp#Settings@1
```

### Resolve the imported side of an alias

The source name of an aliased import identifies the exported symbol.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";
         ^^^^^ imported_name

welcome();
```

```query rename_target main.tspp#imported_name
@rename_target.target placeholder=greet location=main.tspp#imported_name symbol=library.tspp#greet@1
```

### Resolve a namespace re-export alias

A namespace re-export alias remains the rename target when imported by name.

```tspp base.tspp
export function ping(): void {}
```

```tspp barrel.tspp
export * as api from "./base.tspp";
            ^^^ declaration
```

```tspp main.tspp
import { api } from "./barrel.tspp";
         ^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=api location=main.tspp#reference symbol=barrel.tspp#api@1
```

## Export Declarations

### Resolve a default export declaration

A named default export declaration is a rename target.

```tspp main.tspp
export default function greet(): void {}
                        ^^^^^ definition
```

```query rename_target main.tspp#definition
@rename_target.target placeholder=greet location=main.tspp#definition symbol=main.tspp#greet@1
```

## Invalid Targets

### Reject a literal

A literal is not a rename target.

```tspp main.tspp
const count = 42;
              ^^ literal
```

```query rename_target main.tspp#literal
@rename_target.none
```

### Reject a keyword

A keyword is not a rename target.

```tspp main.tspp
function value(): int32 {
    return 1;
    ^^^^^^ keyword
}
```

```query rename_target main.tspp#keyword
@rename_target.none
```

### Reject the constructor keyword

The `constructor` keyword is fixed language syntax rather than a renameable identifier.

```tspp main.tspp
class User {
    constructor(name: string) {}
    ^^^^^^^^^^^ keyword
}
```

```query rename_target main.tspp#keyword
@rename_target.none
```

### Reject a union member shared by unrelated declarations

A shared occurrence with multiple member identities has no unambiguous rename target.

```tspp main.tspp
class Alpha {
    run(): void {}
}

class Beta {
    run(): void {}
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ reference
}
```

```query rename_target main.tspp#reference
@rename_target.none
```

### Reject an unresolved name

An unresolved occurrence has no rename identity.

```tspp main.tspp
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query rename_target main.tspp#reference
@rename_target.none
```

## Associated Types

### Resolve an associated type projection

An associated type projection identifies its member declaration.

```tspp main.tspp
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
}

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference
```

```query rename_target main.tspp#reference
@rename_target.target placeholder=Label location=main.tspp#reference symbol=main.tspp#Label@3
```
