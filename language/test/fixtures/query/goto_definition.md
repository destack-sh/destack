# Goto Definition

## Local Variables

### Resolve a local variable definition

A variable reference resolves to its declaration.

```ds main.ds
const foo = 1;
      ^^^ definition:foo

const bar = foo;
            ^^^ reference:foo
```

```query goto_definition main.ds#reference:foo
@goto_definition.target relation=definition location=main.ds#definition:foo symbol=main.ds#foo@1
```

### Resolve a function parameter definition

A parameter reference resolves to its declaration.

```ds main.ds
function add(x: int32, y: int32): int32 {
             ^ definition:x
    return x + y;
           ^ reference:x
}
```

```query goto_definition main.ds#reference:x
@goto_definition.target relation=definition location=main.ds#definition:x symbol=main.ds#x@2
```

## Functions

### Resolve a function definition

A function call resolves to its declaration.

```ds main.ds
function greet(name: string): string {
         ^^^^^ definition:greet
    return name;
}

const message = greet("World");
                ^^^^^ reference:greet
```

```query goto_definition main.ds#reference:greet
@goto_definition.target relation=definition location=main.ds#definition:greet symbol=main.ds#greet@1
```

## Struct Fields

### Go to a struct field definition

A field access resolves to its field declaration.

```ds main.ds
struct Point {
    x: int32;
    ^ definition:point_x
    y: int32;
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
                    ^ reference:point_x
}
```

```query goto_definition main.ds#reference:point_x
@goto_definition.target relation=definition location=main.ds#definition:point_x symbol=main.ds#x@2
```

## Class Methods

### Go to a class method definition

A method call resolves to its method declaration.

```ds main.ds
class Logger {
    log(message: string): void {
    ^^^ definition:logger_log
    }
}

function main() {
    const logger = new Logger();
    logger.log("hello");
           ^^^ reference:logger_log
}
```

```query goto_definition main.ds#reference:logger_log
@goto_definition.target relation=definition location=main.ds#definition:logger_log symbol=main.ds#log@2
```

## Static Methods

### Resolve a static method definition

A call through a nominal type resolves to the selected static member.

```ds main.ds
class Arithmetic {
    static twice(value: int32): int32 {
           ^^^^^ definition:twice
        return value * 2;
    }
}

const result = Arithmetic.twice(2);
                          ^^^^^ reference:twice
```

```query goto_definition main.ds#reference:twice
@goto_definition.target relation=definition location=main.ds#definition:twice symbol=main.ds#twice@2
```

### Resolve every exact method selected through a union

A union receiver returns the finite member set selected by checking.

```ds main.ds
class Alpha {
    run(): void {}
    ^^^ definition:alpha_run
}

class Beta {
    run(): void {}
    ^^^ definition:beta_run
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ reference
}
```

```query goto_definition main.ds#reference
@goto_definition.target relation=definition location=main.ds#definition:alpha_run symbol=main.ds#run@2
@goto_definition.target relation=definition location=main.ds#definition:beta_run symbol=main.ds#run@5
```

## Extension Methods

### Go to an extension method definition

An extension method call resolves to its extension declaration.

```ds main.ds
struct Calculator {}

extension of Calculator {
    add(x: int32, y: int32): int32 {
    ^^^ definition:calculator_add
        return x + y;
    }
}

function main() {
    const calculator = Calculator {};
    const result = calculator.add(1, 2);
                              ^^^ reference:calculator_add
}
```

```query goto_definition main.ds#reference:calculator_add
@goto_definition.target relation=definition location=main.ds#definition:calculator_add symbol=main.ds#add@3
```

## Enum Members

### Go to an enum member definition

An enum member access resolves to its member declaration.

```ds main.ds
enum Status {
    Pending,
    ^^^^^^^ definition:pending
    Active,
}

const current = Status.Pending;
                       ^^^^^^^ reference:pending
```

```query goto_definition main.ds#reference:pending
@goto_definition.target relation=definition location=main.ds#definition:pending symbol=main.ds#Pending@2
```

## Associated Constants

### Resolve an associated constant definition

A nominal associated constant access resolves to its declaration.

```ds main.ds
struct Buffer {
    comptime const Width: uint = 8;
                   ^^^^^ definition:width
}

const width = Buffer.Width;
                     ^^^^^ reference:width
```

```query goto_definition main.ds#reference:width
@goto_definition.target relation=definition location=main.ds#definition:width symbol=main.ds#Width@2
```

## Type and Value Symbols

### Resolve imported types and values in one module

Imported types and values resolve independently.

```ds types.ds
export type Options = {
            ^^^^^^^ definition:type_options
    enabled: boolean,
};
```

```ds values.ds
export const optionsValue = 1;
             ^^^^^^^^^^^^ definition:value_options
```

```ds main.ds
import { Options } from "./types.ds";
         ^^^^^^^ reference:type_options
import { optionsValue } from "./values.ds";

const typed: Options = { enabled: true };
const value = optionsValue;
              ^^^^^^^^^^^^ reference:value_options
```

```query goto_definition main.ds#reference:type_options
@goto_definition.target relation=definition location=types.ds#definition:type_options symbol=types.ds#Options@1
```

```query goto_definition main.ds#reference:value_options
@goto_definition.target relation=definition location=values.ds#definition:value_options symbol=values.ds#optionsValue@1
```

### Resolve definition through aliased imports

An import alias resolves to the exported declaration.

```ds alias_library.ds
export function greetAliasSource(name: string): string {
                ^^^^^^^^^^^^^^^^ definition:greet_alias_source
    return name;
}
```

```ds alias_main.ds
import { greetAliasSource as localGreeting } from "./alias_library.ds";

const message = localGreeting("Destack");
                ^^^^^^^^^^^^^ reference:local_greeting
```

```query goto_definition alias_main.ds#reference:local_greeting
@goto_definition.target relation=definition location=alias_library.ds#definition:greet_alias_source symbol=alias_library.ds#greetAliasSource@1
```

## Associated Types

### [ignored] Go to an associated type definition

An associated type projection resolves to the associated declaration.

```ds main.ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
         ^^^^^ definition:associated_label
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference:associated_label
```

```query goto_definition main.ds#reference:associated_label
@goto_definition.target relation=definition location=main.ds#definition:associated_label symbol=main.ds#Label@3
```

## Overloads

### Go to the matching overload

Each overload call resolves to the matching declaration.

```ds main.ds
function parse(value: int32): int32 {
         ^^^^^ definition:parse_integer
    return value;
}

function parse(value: string): string {
         ^^^^^ definition:parse_string
    return value;
}

const integerValue = parse(1);
                     ^^^^^ reference:parse_integer
const textValue = parse("ok");
                  ^^^^^ reference:parse_string
```

```query goto_definition main.ds#reference:parse_integer
@goto_definition.target relation=definition location=main.ds#definition:parse_integer symbol=main.ds#parse@1
```

```query goto_definition main.ds#reference:parse_string
@goto_definition.target relation=definition location=main.ds#definition:parse_string symbol=main.ds#parse@3
```

## Construction

### Resolve class construction to the class definition

Definition navigation on a class name selects the class rather than the constructor hierarchy item.

```ds main.ds
class User {
      ^^^^ definition:user
    constructor(name: string) {}
}

const user = new User("Ada");
                 ^^^^ reference:user
```

```query goto_definition main.ds#reference:user
@goto_definition.target relation=definition location=main.ds#definition:user symbol=main.ds#User@1
```

### Resolve newtype construction to the newtype definition

A newtype call retains the nominal declaration used by its authored name.

```ds main.ds
newtype UserId = string;
        ^^^^^^ definition:user_id

const userId = UserId("user-1");
               ^^^^^^ reference:user_id
```

```query goto_definition main.ds#reference:user_id
@goto_definition.target relation=definition location=main.ds#definition:user_id symbol=main.ds#UserId@1
```

### Resolve tagged construction to the variant definition

A tagged construction selects the exact generated variant represented by its authored case name.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;
                 ^^ definition:ok

const status = Status.Ok({ value: "ready" });
                      ^^ reference:ok
```

```query goto_definition main.ds#reference:ok
@goto_definition.target relation=definition location=main.ds#definition:ok symbol=main.ds#Ok@3
```

## Imports and Exports

### Go to a definition through a namespace re-export

An export namespace resolves member references through the re-export.

```ds base.ds
export function ping(): void {}
                ^^^^ definition:namespace_export_target
```

```ds barrel.ds
export * as api from "./base.ds";
```

```ds main.ds
import { api } from "./barrel.ds";

api.ping();
    ^^^^ reference:namespace_export_target
```

```query goto_definition main.ds#reference:namespace_export_target
@goto_definition.target relation=definition location=base.ds#definition:namespace_export_target symbol=base.ds#ping@1
```

### Go to a definition through a default re-export alias

A default re-export alias resolves to its source declaration.

```ds library.ds
export default function buildWidget(): int32 {
                        ^^^^^^^^^^^ definition:buildWidget
    return 1;
}
```

```ds barrel.ds
export { default as buildWidget } from "./library.ds";
```

```ds main.ds
import { buildWidget } from "./barrel.ds";

const value = buildWidget();
              ^^^^^^^^^^^ reference:buildWidget
```

```query goto_definition main.ds#reference:buildWidget
@goto_definition.target relation=definition location=library.ds#definition:buildWidget symbol=library.ds#buildWidget@1
```

## Imported Functions

### Resolve an imported function

An imported call resolves to the exported function declaration.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ definition:imported_function
    return name;
}
```

```ds main.ds
import { greet } from "./library.ds";

const message = greet("Destack");
                ^^^^^ reference:imported_function
```

```query goto_definition main.ds#reference:imported_function
@goto_definition.target relation=definition location=library.ds#definition:imported_function symbol=library.ds#greet@1
```

### Resolve an aliased import

An import alias resolves to the exported function declaration.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ definition:aliased_import
    return name;
}
```

```ds main.ds
import { greet as welcome } from "./library.ds";

const message = welcome("Destack");
                ^^^^^^^ reference:aliased_import
```

```query goto_definition main.ds#reference:aliased_import
@goto_definition.target relation=definition location=library.ds#definition:aliased_import symbol=library.ds#greet@1
```

## Re-Exports

### Resolve a named re-export

A reference through a named re-export resolves to the original declaration.

```ds library.ds
export function greet(name: string): string {
                ^^^^^ definition:named_reexport
    return name;
}
```

```ds public.ds
export { greet } from "./library.ds";
```

```ds main.ds
import { greet } from "./public.ds";

const message = greet("Destack");
                ^^^^^ reference:named_reexport
```

```query goto_definition main.ds#reference:named_reexport
@goto_definition.target relation=definition location=library.ds#definition:named_reexport symbol=library.ds#greet@1
```

### Resolve a default re-export alias

A reference through a default re-export alias resolves to the original declaration.

```ds library.ds
export default function buildWidget(): int32 {
                        ^^^^^^^^^^^ definition:default_reexport
    return 1;
}
```

```ds public.ds
export { default as buildWidget } from "./library.ds";
```

```ds main.ds
import { buildWidget } from "./public.ds";

const value = buildWidget();
              ^^^^^^^^^^^ reference:default_reexport
```

```query goto_definition main.ds#reference:default_reexport
@goto_definition.target relation=definition location=library.ds#definition:default_reexport symbol=library.ds#buildWidget@1
```

## Imported Types

### Resolve an imported type

A plain import preserves the exported declaration's type symbol space.

```ds model.ds
export type Options = {
            ^^^^^^^ definition:imported_type
    enabled: boolean;
};
```

```ds main.ds
import { Options } from "./model.ds";

const options: Options = { enabled: true };
               ^^^^^^^ reference:imported_type
```

```query goto_definition main.ds#reference:imported_type
@goto_definition.target relation=definition location=model.ds#definition:imported_type symbol=model.ds#Options@1
```

### Resolve an imported type alias

An aliased import preserves the exported declaration's type symbol space.

```ds model.ds
export type Options = {
            ^^^^^^^ definition:aliased_type
    enabled: boolean;
};
```

```ds main.ds
import { Options as Configuration } from "./model.ds";

const options: Configuration = { enabled: true };
               ^^^^^^^^^^^^^ reference:aliased_type
```

```query goto_definition main.ds#reference:aliased_type
@goto_definition.target relation=definition location=model.ds#definition:aliased_type symbol=model.ds#Options@1
```

## Missing Symbols

### Return no definition for an unresolved name

An unresolved name has no definition.

```ds main.ds
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query goto_definition main.ds#reference
@goto_definition.none
```

## Neighboring Diagnostics

### Resolve a symbol beside an unresolved name

An unrelated diagnostic does not change the symbol identity.

```ds main.ds
const value = 1;
      ^^^^^ definition:value

function main(): int32 {
    missingValue;
    return value;
           ^^^^^ reference:value
}
```

```query goto_definition main.ds#reference:value
@goto_definition.target relation=definition location=main.ds#definition:value symbol=main.ds#value@1
```

## Pattern Bindings

### Resolve a match binding definition

A match-arm reference resolves to the binding introduced by its pattern.

```ds main.ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ definition:match_left
                    ^^^^ reference:match_left
};
```

```query goto_definition main.ds#reference:match_left
@goto_definition.target relation=definition location=main.ds#definition:match_left symbol=main.ds#left@3
```

## Labels

### Resolve a control label definition

A labeled break resolves to the exact enclosing label.

```ds main.ds
function choose(): int32 {
    outer: loop {
    ^^^^^ definition:outer
        break outer: 1;
              ^^^^^ reference:outer
    }
}
```

```query goto_definition main.ds#reference:outer
@goto_definition.target relation=definition location=main.ds#definition:outer symbol=main.ds#outer@2
```

## Calls

### Resolve a tagged-template function definition

A tagged template resolves its tag like an ordinary call.

```ds main.ds
function sql(parts: string[], ...values: int32): string {
         ^^^ definition:sql
    return "";
}

const query = sql`select ${1}`;
              ^^^ reference:sql
```

```query goto_definition main.ds#reference:sql
@goto_definition.target relation=definition location=main.ds#definition:sql symbol=main.ds#sql@1
```

### Resolve a comptime call definition

A comptime call retains the called function identity.

```ds main.ds
function build(): int32 {
         ^^^^^ definition:build
    return 1;
}

const value = comptime build();
                       ^^^^^ reference:build
```

```query goto_definition main.ds#reference:build
@goto_definition.target relation=definition location=main.ds#definition:build symbol=main.ds#build@1
```

## Using Bindings

### Resolve a using binding definition

A using reference resolves to its lexical binding.

```ds main.ds
declare function openSession(): Dispose;

using session = openSession();
      ^^^^^^^ definition:session

const value = session;
              ^^^^^^^ reference:session
```

```query goto_definition main.ds#reference:session
@goto_definition.target relation=definition location=main.ds#definition:session symbol=main.ds#session@2
```

## Annotations

### Resolve an annotation definition

An annotation application resolves through ordinary value identity.

```ds main.ds
newtype tracked = ();
        ^^^^^^^ definition:tracked

@tracked
 ^^^^^^^ reference:tracked
class Service {}
```

```query goto_definition main.ds#reference:tracked
@goto_definition.target relation=definition location=main.ds#definition:tracked symbol=main.ds#tracked@1
```
