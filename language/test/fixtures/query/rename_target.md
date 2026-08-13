
## Local Bindings

### Resolve a definition

A definition identifies its symbol and placeholder name.

```ds main.ds
const count = 1;
      ^^^^^ definition
const next = count;
```

```query rename_target main.ds#definition
@rename_target.target placeholder=count location=main.ds#definition symbol=main.ds#count@1
```

### Resolve a reference

A reference reports its name range and symbol.

```ds main.ds
const count = 1;
const next = count;
             ^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=count location=main.ds#reference symbol=main.ds#count@1
```

## Members

### Resolve a field access

A named field access resolves as a rename target.

```ds main.ds
struct Counter {
    count: int32;
}

function read(counter: Counter): int32 {
    return counter.count;
                   ^^^^^ reference
}
```

```query rename_target main.ds#reference
@rename_target.target placeholder=count location=main.ds#reference symbol=main.ds#count@2
```

### Resolve a string-keyed field access

A string-keyed field access exposes only the identifier contents as its rename range.

```ds main.ds
struct Counter {
    count: int32;
}

function read(counter: Counter): int32 {
    return counter["count"];
                    ^^^^^ reference
}
```

```query rename_target main.ds#reference
@rename_target.target placeholder=count location=main.ds#reference symbol=main.ds#count@2
```

## Methods

### Resolve a method access

A method name is a rename target.

```ds main.ds
class Service {
    run(): void {}
}

function start(service: Service): void {
    service.run();
            ^^^ reference
}
```

```query rename_target main.ds#reference
@rename_target.target placeholder=run location=main.ds#reference symbol=main.ds#run@2
```

### Resolve an extension method access

An extension call identifies its extension method.

```ds main.ds
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

```query rename_target main.ds#reference
@rename_target.target placeholder=add location=main.ds#reference symbol=main.ds#add@3
```

## Associated Constants

### Resolve an associated constant access

An associated constant identifies its member declaration.

```ds main.ds
struct Buffer {
    const Width: uint = 8;
}

const width = Buffer.Width;
                     ^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=Width location=main.ds#reference symbol=main.ds#Width@2
```

## Enum Members

### Resolve an enum member access

An enum member occurrence identifies its member declaration.

```ds main.ds
enum Color {
    Red,
}

const color = Color.Red;
                    ^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=Red location=main.ds#reference symbol=main.ds#Red@2
```

## Member Declarations

### Resolve a field definition

A member name is a rename target.

```ds main.ds
struct Point {
    x: int32;
    ^ definition
}
```

```query rename_target main.ds#definition
@rename_target.target placeholder=x location=main.ds#definition symbol=main.ds#x@2
```

## Generic Parameters

### Resolve a type parameter reference

A generic type reference identifies its local parameter.

```ds main.ds
function identity<T>(value: T): T {
                                ^ reference
    return value;
}
```

```query rename_target main.ds#reference
@rename_target.target placeholder=T location=main.ds#reference symbol=main.ds#T@2
```

### Resolve a type parameter in a template literal

A type parameter inside a template literal identifies its local parameter.

```ds main.ds
type Route<T: string> = `api:${T}`;
                               ^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=T location=main.ds#reference symbol=main.ds#T@2
```

### Resolve a const type parameter

A const type parameter identifies its local parameter.

```ds main.ds
type Buffer<const size: usize> = [uint8; size];
                                            ^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=size location=main.ds#reference symbol=main.ds#size@2
```

## Pattern Bindings

### Resolve a destructured binding

A destructured name is an ordinary lexical rename target.

```ds main.ds
const pair = { left: 1, right: 2 };
const { left } = pair;
        ^^^^ definition

const value = left;
              ^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=left location=main.ds#reference symbol=main.ds#left@2
```

### Resolve a match binding

A match-arm name identifies its arm-local binding.

```ds main.ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ definition
                     ^^^^ reference
};
```

```query rename_target main.ds#reference
@rename_target.target placeholder=left location=main.ds#reference symbol=main.ds#left@2
```

### Resolve the local value of an object shorthand

An object shorthand token selects its local value binding for rename preparation.

```ds main.ds
const horizontal = 1;

const point = { horizontal };
                ^^^^^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=horizontal location=main.ds#reference symbol=main.ds#horizontal@1
```

## Overloads

### Resolve an overload family at a call

Rename preparation identifies every declaration that shares the called function name.

```ds main.ds
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const value = parse(1);
              ^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=parse location=main.ds#reference symbols=main.ds#parse@1,main.ds#parse@3
```

## Construction

### Resolve a class construction name

The class name in a construction expression is a rename target for the class identity.

```ds main.ds
class User {
    constructor(name: string) {}
}

const user = new User("Ada");
                 ^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=User location=main.ds#reference symbol=main.ds#User@1
```

## Labels

### Resolve a control label reference

A targeted break identifies its enclosing label.

```ds main.ds
function choose(): int32 {
    outer: loop {
        break outer: 1;
              ^^^^^ reference
    }
}
```

```query rename_target main.ds#reference
@rename_target.target placeholder=outer location=main.ds#reference symbol=main.ds#outer@2
```

## Calls

### Resolve a tagged-template function

A tagged-template tag is the same rename target as an ordinary call.

```ds main.ds
function sql(parts: string[], ...values: int32[]): string {
    return "";
}

const query = sql`select ${1}`;
              ^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=sql location=main.ds#reference symbol=main.ds#sql@1
```

### Resolve a const call

A const call identifies the called function.

```ds main.ds
function build(): int32 {
    return 1;
}

const value = const build();
                       ^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=build location=main.ds#reference symbol=main.ds#build@1
```

## Annotations

### Resolve an annotation application

An annotation value is a rename target at its application.

```ds main.ds
newtype tracked = ();

@tracked
 ^^^^^^^ reference
class Service {}
```

```query rename_target main.ds#reference
@rename_target.target placeholder=tracked location=main.ds#reference symbol=main.ds#tracked@1
```

### Resolve a parameter annotation

An annotation on a parameter identifies the annotation value.

```ds main.ds
newtype tracked = ();

function start(@tracked value: int32): void {}
                ^^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=tracked location=main.ds#reference symbol=main.ds#tracked@1
```

## Using Bindings

### Resolve a using binding

A using declaration introduces an ordinary lexical rename target.

```ds main.ds
declare function openSession(): Dispose;

using session = openSession();

const value = session;
              ^^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=session location=main.ds#reference symbol=main.ds#session@2
```

## Import Bindings

### Resolve a local import alias

An explicit import alias remains a local rename target.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { greet as welcome } from "./library.ds";

welcome();
^^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=welcome location=main.ds#reference symbol=main.ds#welcome@1
```

### Resolve a namespace import alias

A namespace receiver identifies its local import alias.

```ds library.ds
export function ping(): void {}
```

```ds main.ds
import * as api from "./library.ds";

api.ping();
^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=api location=main.ds#reference symbol=main.ds#api@1
```

### Resolve a default import binding

A default import name is local to the importing module.

```ds library.ds
export default function greet(): void {}
```

```ds main.ds
import welcome from "./library.ds";

welcome();
^^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=welcome location=main.ds#reference symbol=main.ds#welcome@1
```

### Resolve an imported type alias

An explicit local alias remains a rename target in the declaration's type symbol space.

```ds library.ds
export type Options = {
    enabled: boolean,
};
```

```ds main.ds
import { Options as Settings } from "./library.ds";

declare const settings: Settings;
                        ^^^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=Settings location=main.ds#reference symbol=main.ds#Settings@1
```

### Resolve the imported side of an alias

The source name of an aliased import identifies the exported symbol.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { greet as welcome } from "./library.ds";
         ^^^^^ imported_name

welcome();
```

```query rename_target main.ds#imported_name
@rename_target.target placeholder=greet location=main.ds#imported_name symbol=library.ds#greet@1
```

### Resolve a namespace re-export alias

A namespace re-export alias remains the rename target when imported by name.

```ds base.ds
export function ping(): void {}
```

```ds barrel.ds
export * as api from "./base.ds";
            ^^^ declaration
```

```ds main.ds
import { api } from "./barrel.ds";
         ^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=api location=main.ds#reference symbol=barrel.ds#api@1
```

## Export Declarations

### Resolve a default export declaration

A named default export declaration is a rename target.

```ds main.ds
export default function greet(): void {}
                        ^^^^^ definition
```

```query rename_target main.ds#definition
@rename_target.target placeholder=greet location=main.ds#definition symbol=main.ds#greet@1
```

## Invalid Targets

### Reject a literal

A literal is not a rename target.

```ds main.ds
const count = 42;
              ^^ literal
```

```query rename_target main.ds#literal
@rename_target.none
```

### Reject a keyword

A keyword is not a rename target.

```ds main.ds
function value(): int32 {
    return 1;
    ^^^^^^ keyword
}
```

```query rename_target main.ds#keyword
@rename_target.none
```

### Reject the constructor keyword

The constructor role is fixed language syntax rather than a renameable identifier.

```ds main.ds
class User {
    constructor(name: string) {}
    ^^^^^^^^^^^ keyword
}
```

```query rename_target main.ds#keyword
@rename_target.none
```

### Reject a union member shared by unrelated declarations

A shared occurrence with multiple member identities has no unambiguous rename target.

```ds main.ds
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

```query rename_target main.ds#reference
@rename_target.none
```

### Reject an unresolved name

An unresolved occurrence has no rename identity.

```ds main.ds
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query rename_target main.ds#reference
@rename_target.none
```

## Associated Types

### Resolve an associated type projection

An associated type projection identifies its member declaration.

```ds main.ds
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
}

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=Label location=main.ds#reference symbol=main.ds#Label@3
```

## Source changes

### Follow a reference after its binding changes

The rename target is the symbol selected in each revision.

```ds main.ds
const first = 1;
const second = 2;
const selected = first;
                 ^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=first location=main.ds#reference symbol=main.ds#first@1
```

```ds main.ds change
const first = 1;
const second = 2;
const selected = second;
                 ^^^^^^ reference
```

```query rename_target main.ds#reference
@rename_target.target placeholder=second location=main.ds#reference symbol=main.ds#second@2
```
