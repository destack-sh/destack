
## Local Variables

### Resolve a local variable definition

A variable reference resolves to its declaration.

```tspp main.tspp
const foo = 1;
      ^^^ definition:foo

const bar = foo;
            ^^^ reference:foo
```

```query goto_definition main.tspp#reference:foo
@goto_definition.target origin=main.tspp#reference:foo location=main.tspp#definition:foo symbol=main.tspp#foo@1
```

### Resolve a function parameter definition

A parameter reference resolves to its declaration.

```tspp main.tspp
function add(x: int32, y: int32): int32 {
             ^ definition:x
             ^^^^^^^^ declaration:x
    return x + y;
           ^ reference:x
}
```

```query goto_definition main.tspp#reference:x
@goto_definition.target origin=main.tspp#reference:x location=main.tspp#declaration:x selection=main.tspp#definition:x symbol=main.tspp#x@2
```

## Functions

### Resolve a function definition

A function call resolves to its declaration.

```tspp main.tspp
function greet(name: string): string {
^ declaration:greet:start
         ^^^^^ definition:greet
    return name;
}
^ declaration:greet:end

const message = greet("World");
                ^^^^^ reference:greet
```

```query goto_definition main.tspp#reference:greet
@goto_definition.target origin=main.tspp#reference:greet location=main.tspp#declaration:greet selection=main.tspp#definition:greet symbol=main.tspp#greet@1
```

## Struct Fields

### Go to a struct field definition

A field access resolves to its field declaration.

```tspp main.tspp
struct Point {
    x: int32;
    ^ definition:point_x
    ^^^^^^^^ declaration:point_x
    y: int32;
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
                    ^ reference:point_x
}
```

```query goto_definition main.tspp#reference:point_x
@goto_definition.target origin=main.tspp#reference:point_x location=main.tspp#declaration:point_x selection=main.tspp#definition:point_x symbol=main.tspp#x@2
```

## Class Methods

### Go to a class method definition

A method call resolves to its method declaration.

```tspp main.tspp
class Logger {
    log(message: string): void {
    ^ declaration:logger_log:start
    ^^^ definition:logger_log
    }
    ^ declaration:logger_log:end
}

function main() {
    const logger = new Logger();
    logger.log("hello");
           ^^^ reference:logger_log
}
```

```query goto_definition main.tspp#reference:logger_log
@goto_definition.target origin=main.tspp#reference:logger_log location=main.tspp#declaration:logger_log selection=main.tspp#definition:logger_log symbol=main.tspp#log@2
```

## Static Methods

### Resolve a static method definition

A call through a nominal type resolves to its static member.

```tspp main.tspp
class Arithmetic {
    static twice(value: int32): int32 {
    ^ declaration:twice:start
           ^^^^^ definition:twice
        return value * 2;
    }
    ^ declaration:twice:end
}

const result = Arithmetic.twice(2);
                          ^^^^^ reference:twice
```

```query goto_definition main.tspp#reference:twice
@goto_definition.target origin=main.tspp#reference:twice location=main.tspp#declaration:twice selection=main.tspp#definition:twice symbol=main.tspp#twice@2
```

### Resolve every method reached through a union

A union receiver returns every method declaration available at that call.

```tspp main.tspp
class Alpha {
    run(): void {}
    ^^^ definition:alpha_run
    ^^^^^^^^^^^^^^ declaration:alpha_run
}

class Beta {
    run(): void {}
    ^^^ definition:beta_run
    ^^^^^^^^^^^^^^ declaration:beta_run
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ reference
}
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=main.tspp#declaration:alpha_run selection=main.tspp#definition:alpha_run symbol=main.tspp#run@2
@goto_definition.target origin=main.tspp#reference location=main.tspp#declaration:beta_run selection=main.tspp#definition:beta_run symbol=main.tspp#run@5
```

## Extension Methods

### Go to an extension method definition

An extension method call resolves to its extension declaration.

```tspp main.tspp
struct Calculator {}

extension of Calculator {
    add(x: int32, y: int32): int32 {
    ^ declaration:calculator_add:start
    ^^^ definition:calculator_add
        return x + y;
    }
    ^ declaration:calculator_add:end
}

function main() {
    const calculator = Calculator {};
    const result = calculator.add(1, 2);
                              ^^^ reference:calculator_add
}
```

```query goto_definition main.tspp#reference:calculator_add
@goto_definition.target origin=main.tspp#reference:calculator_add location=main.tspp#declaration:calculator_add selection=main.tspp#definition:calculator_add symbol=main.tspp#add@3
```

## Enum Members

### Go to an enum member definition

An enum member access resolves to its member declaration.

```tspp main.tspp
enum Status {
    Pending,
    ^^^^^^^ definition:pending
    Active,
}

const current = Status.Pending;
                       ^^^^^^^ reference:pending
```

```query goto_definition main.tspp#reference:pending
@goto_definition.target origin=main.tspp#reference:pending location=main.tspp#definition:pending symbol=main.tspp#Pending@2
```

## Associated Constants

### Resolve an associated constant definition

A nominal associated constant access resolves to its declaration.

```tspp main.tspp
struct Buffer {
    const Width: uint64 = 8;
          ^^^^^ definition:width
    ^^^^^^^^^^^^^^^^^^^^^^^ width_declaration
}

const width = Buffer.Width;
                     ^^^^^ reference:width
```

```query goto_definition main.tspp#reference:width
@goto_definition.target origin=main.tspp#reference:width location=main.tspp#width_declaration selection=main.tspp#definition:width symbol=main.tspp#Width@2
```

## Type and Value Symbols

### Resolve imported types and values in one module

Imported types and values resolve independently.

```tspp types.tspp
export type Options = {
^ declaration:type_options:start
            ^^^^^^^ definition:type_options
    enabled: boolean,
};
^ declaration:type_options:end
```

```tspp values.tspp
export const optionsValue = 1;
             ^^^^^^^^^^^^ definition:value_options
```

```tspp main.tspp
import { Options } from "./types.tspp";
         ^^^^^^^ reference:type_options
import { optionsValue } from "./values.tspp";

const typed: Options = { enabled: true };
const value = optionsValue;
              ^^^^^^^^^^^^ reference:value_options
```

```query goto_definition main.tspp#reference:type_options
@goto_definition.target origin=main.tspp#reference:type_options location=types.tspp#declaration:type_options selection=types.tspp#definition:type_options symbol=types.tspp#Options@1
```

```query goto_definition main.tspp#reference:value_options
@goto_definition.target origin=main.tspp#reference:value_options location=values.tspp#definition:value_options symbol=values.tspp#optionsValue@1
```

### Resolve definition through aliased imports

An import alias resolves to the exported declaration.

```tspp alias_library.tspp
export function greetAliasSource(name: string): string {
^ declaration:greet_alias_source:start
                ^^^^^^^^^^^^^^^^ definition:greet_alias_source
    return name;
}
^ declaration:greet_alias_source:end
```

```tspp alias_main.tspp
import { greetAliasSource as localGreeting } from "./alias_library.tspp";

const message = localGreeting("Destack");
                ^^^^^^^^^^^^^ reference:local_greeting
```

```query goto_definition alias_main.tspp#reference:local_greeting
@goto_definition.target origin=alias_main.tspp#reference:local_greeting location=alias_library.tspp#declaration:greet_alias_source selection=alias_library.tspp#definition:greet_alias_source symbol=alias_library.tspp#greetAliasSource@1
```

## Associated Types

### Go to a path segment definition

Each segment of a qualified type path resolves to its own declaration.

```tspp geometry.tspp
export struct Slot {
    type Value = int32;
    ^^^^^^^^^^^^^^^^^^ declaration:value_member
         ^^^^^ definition:value_member
}

export struct Grid {
^ declaration:grid:start
              ^^^^ definition:grid
    type Cell = Slot;
    ^^^^^^^^^^^^^^^^ declaration:cell_member
         ^^^^ definition:cell_member
}
^ declaration:grid:end
```

```tspp main.tspp
import { Grid } from "./geometry.tspp";

declare const value: Grid.Cell.Value;
                     ^^^^ reference:grid_segment
                          ^^^^ reference:cell_segment
                               ^^^^^ reference:value_segment
```

```query goto_definition main.tspp#reference:grid_segment
@goto_definition.target origin=main.tspp#reference:grid_segment location=geometry.tspp#declaration:grid selection=geometry.tspp#definition:grid symbol=geometry.tspp#Grid@4
```

```query goto_definition main.tspp#reference:cell_segment
@goto_definition.target origin=main.tspp#reference:cell_segment location=geometry.tspp#declaration:cell_member selection=geometry.tspp#definition:cell_member symbol=geometry.tspp#Cell@5
```

```query goto_definition main.tspp#reference:value_segment
@goto_definition.target origin=main.tspp#reference:value_segment location=geometry.tspp#declaration:value_member selection=geometry.tspp#definition:value_member symbol=geometry.tspp#Value@2
```

### Go to an associated type definition

An associated type projection resolves to the associated declaration.

```tspp main.tspp
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:associated_label
         ^^^^^ definition:associated_label
}

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference:associated_label
```

```query goto_definition main.tspp#reference:associated_label
@goto_definition.target origin=main.tspp#reference:associated_label location=main.tspp#declaration:associated_label selection=main.tspp#definition:associated_label symbol=main.tspp#Label@3
```

```diff main.tspp
@@ -1,4 +1,4 @@
 interface Envelope<T: string> {
-    type Label<U: string> = `${T}:${U}`;
+    type Token<U: string> = `${T}:${U}`;
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:associated_label
          ^^^^^ definition:associated_label
@@ -9,2 +9,2 @@
-type EventLabel = Message<"orders">.Label<"created">;
+type EventToken = Message<"orders">.Token<"created">;
                                     ^^^^^ reference:associated_label
```

```query goto_definition main.tspp#reference:associated_label
@goto_definition.target origin=main.tspp#reference:associated_label location=main.tspp#declaration:associated_label selection=main.tspp#definition:associated_label symbol=main.tspp#Token@3
```

### Go to an imported associated type definition

An associated type projection resolves across its imported interface.

```tspp envelope.tspp
export interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:associated_label
         ^^^^^ definition:associated_label
}
```

```tspp message.tspp
import { Envelope } from "./envelope.tspp";

class Message<T: string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
                                    ^^^^^ reference:associated_label
```

```query goto_definition message.tspp#reference:associated_label
@goto_definition.target origin=message.tspp#reference:associated_label location=envelope.tspp#declaration:associated_label selection=envelope.tspp#definition:associated_label symbol=envelope.tspp#Label@3
```

## Overloads

### Go to the matching overload

Each overload call resolves to the matching declaration.

```tspp main.tspp
function parse(value: int32): int32 {
^ declaration:parse_integer:start
         ^^^^^ definition:parse_integer
    return value;
}
^ declaration:parse_integer:end

function parse(value: string): string {
^ declaration:parse_string:start
         ^^^^^ definition:parse_string
    return value;
}
^ declaration:parse_string:end

const integerValue = parse(1);
                     ^^^^^ reference:parse_integer
const textValue = parse("ok");
                  ^^^^^ reference:parse_string
```

```query goto_definition main.tspp#reference:parse_integer
@goto_definition.target origin=main.tspp#reference:parse_integer location=main.tspp#declaration:parse_integer selection=main.tspp#definition:parse_integer symbol=main.tspp#parse@1
```

```query goto_definition main.tspp#reference:parse_string
@goto_definition.target origin=main.tspp#reference:parse_string location=main.tspp#declaration:parse_string selection=main.tspp#definition:parse_string symbol=main.tspp#parse@3
```

## Construction

### Resolve class construction to the class definition

Definition navigation on a class name selects the class rather than the constructor hierarchy item.

```tspp main.tspp
class User {
^ declaration:user:start
      ^^^^ definition:user
    constructor(name: string) {}
}
^ declaration:user:end

const user = new User("Ada");
                 ^^^^ reference:user
```

```query goto_definition main.tspp#reference:user
@goto_definition.target origin=main.tspp#reference:user location=main.tspp#declaration:user selection=main.tspp#definition:user symbol=main.tspp#User@1
```

### Resolve newtype construction to the newtype definition

A newtype call resolves to its nominal declaration.

```tspp main.tspp
newtype UserId = string;
^ declaration:user_id:start
        ^^^^^^ definition:user_id
                      ^ declaration:user_id:end

const userId = UserId("user-1");
               ^^^^^^ reference:user_id
```

```query goto_definition main.tspp#reference:user_id
@goto_definition.target origin=main.tspp#reference:user_id location=main.tspp#declaration:user_id selection=main.tspp#definition:user_id symbol=main.tspp#UserId@1
```

## Imports and Exports

### Go to a definition through a namespace re-export

An export namespace resolves member references through the re-export.

```tspp base.tspp
export function ping(): void {}
^ declaration:namespace_export_target:start
                ^^^^ definition:namespace_export_target
                              ^ declaration:namespace_export_target:end
```

```tspp barrel.tspp
export * as api from "./base.tspp";
```

```tspp main.tspp
import { api } from "./barrel.tspp";

api.ping();
    ^^^^ reference:namespace_export_target
```

```query goto_definition main.tspp#reference:namespace_export_target
@goto_definition.target origin=main.tspp#reference:namespace_export_target location=base.tspp#declaration:namespace_export_target selection=base.tspp#definition:namespace_export_target symbol=base.tspp#ping@1
```

### Go to a definition through a default re-export alias

A default re-export alias resolves to its source declaration.

```tspp library.tspp
export default function buildWidget(): int32 {
^ declaration:buildWidget:start
                        ^^^^^^^^^^^ definition:buildWidget
    return 1;
}
^ declaration:buildWidget:end
```

```tspp barrel.tspp
export { default as buildWidget } from "./library.tspp";
```

```tspp main.tspp
import { buildWidget } from "./barrel.tspp";

const value = buildWidget();
              ^^^^^^^^^^^ reference:buildWidget
```

```query goto_definition main.tspp#reference:buildWidget
@goto_definition.target origin=main.tspp#reference:buildWidget location=library.tspp#declaration:buildWidget selection=library.tspp#definition:buildWidget symbol=library.tspp#buildWidget@1
```

## Imported Functions

### Resolve an imported function

An imported call resolves to the exported function declaration.

```tspp library.tspp
export function greet(name: string): string {
^ declaration:imported_function:start
                ^^^^^ definition:imported_function
    return name;
}
^ declaration:imported_function:end
```

```tspp main.tspp
import { greet } from "./library.tspp";

const message = greet("Destack");
                ^^^^^ reference:imported_function
```

```query goto_definition main.tspp#reference:imported_function
@goto_definition.target origin=main.tspp#reference:imported_function location=library.tspp#declaration:imported_function selection=library.tspp#definition:imported_function symbol=library.tspp#greet@1
```

### Resolve an aliased import

An import alias resolves to the exported function declaration.

```tspp library.tspp
export function greet(name: string): string {
^ declaration:aliased_import:start
                ^^^^^ definition:aliased_import
    return name;
}
^ declaration:aliased_import:end
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";

const message = welcome("Destack");
                ^^^^^^^ reference:aliased_import
```

```query goto_definition main.tspp#reference:aliased_import
@goto_definition.target origin=main.tspp#reference:aliased_import location=library.tspp#declaration:aliased_import selection=library.tspp#definition:aliased_import symbol=library.tspp#greet@1
```

### Resolve the current imported definition

An imported reference follows the declaration selected after each edit.

```tspp library.tspp
export function greet(name: string): string {
^ declaration:start
                ^^^^^ definition
    return name;
}
^ declaration:end
```

```tspp main.tspp
import { greet } from "./library.tspp";

const message = greet("World");
                ^^^^^ reference
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=library.tspp#declaration selection=library.tspp#definition symbol=library.tspp#greet@1
```

```tspp library.tspp change
export const prefix = "Hello";

export function greet(name: string): string {
^ declaration:start
                ^^^^^ definition
    return prefix;
}
^ declaration:end
```

```query goto_definition main.tspp#reference
@goto_definition.target origin=main.tspp#reference location=library.tspp#declaration selection=library.tspp#definition symbol=library.tspp#greet@2
```

## Re-Exports

### Resolve a named re-export

A reference through a named re-export resolves to the original declaration.

```tspp library.tspp
export function greet(name: string): string {
^ declaration:named_reexport:start
                ^^^^^ definition:named_reexport
    return name;
}
^ declaration:named_reexport:end
```

```tspp public.tspp
export { greet } from "./library.tspp";
```

```tspp main.tspp
import { greet } from "./public.tspp";

const message = greet("Destack");
                ^^^^^ reference:named_reexport
```

```query goto_definition main.tspp#reference:named_reexport
@goto_definition.target origin=main.tspp#reference:named_reexport location=library.tspp#declaration:named_reexport selection=library.tspp#definition:named_reexport symbol=library.tspp#greet@1
```

### Resolve a default re-export alias

A reference through a default re-export alias resolves to the original declaration.

```tspp library.tspp
export default function buildWidget(): int32 {
^ declaration:default_reexport:start
                        ^^^^^^^^^^^ definition:default_reexport
    return 1;
}
^ declaration:default_reexport:end
```

```tspp public.tspp
export { default as buildWidget } from "./library.tspp";
```

```tspp main.tspp
import { buildWidget } from "./public.tspp";

const value = buildWidget();
              ^^^^^^^^^^^ reference:default_reexport
```

```query goto_definition main.tspp#reference:default_reexport
@goto_definition.target origin=main.tspp#reference:default_reexport location=library.tspp#declaration:default_reexport selection=library.tspp#definition:default_reexport symbol=library.tspp#buildWidget@1
```

## Imported Types

### Resolve an imported type

A plain import preserves the exported declaration's type symbol space.

```tspp model.tspp
export type Options = {
^ declaration:imported_type:start
            ^^^^^^^ definition:imported_type
    enabled: boolean;
};
^ declaration:imported_type:end
```

```tspp main.tspp
import { Options } from "./model.tspp";

const options: Options = { enabled: true };
               ^^^^^^^ reference:imported_type
```

```query goto_definition main.tspp#reference:imported_type
@goto_definition.target origin=main.tspp#reference:imported_type location=model.tspp#declaration:imported_type selection=model.tspp#definition:imported_type symbol=model.tspp#Options@1
```

### Resolve an imported type alias

An aliased import preserves the exported declaration's type symbol space.

```tspp model.tspp
export type Options = {
^ declaration:aliased_type:start
            ^^^^^^^ definition:aliased_type
    enabled: boolean;
};
^ declaration:aliased_type:end
```

```tspp main.tspp
import { Options as Configuration } from "./model.tspp";

const options: Configuration = { enabled: true };
               ^^^^^^^^^^^^^ reference:aliased_type
```

```query goto_definition main.tspp#reference:aliased_type
@goto_definition.target origin=main.tspp#reference:aliased_type location=model.tspp#declaration:aliased_type selection=model.tspp#definition:aliased_type symbol=model.tspp#Options@1
```

## Missing Symbols

### Return no definition for an unresolved name

An unresolved name has no definition.

```tspp main.tspp
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query goto_definition main.tspp#reference
@goto_definition.none
```

## Neighboring Diagnostics

### Resolve a symbol beside an unresolved name

An unrelated diagnostic does not change the symbol identity.

```tspp main.tspp
const value = 1;
      ^^^^^ definition:value

function main(): int32 {
    missingValue;
    return value;
           ^^^^^ reference:value
}
```

```query goto_definition main.tspp#reference:value
@goto_definition.target origin=main.tspp#reference:value location=main.tspp#definition:value symbol=main.tspp#value@1
```

## Pattern Bindings

### Resolve a match binding definition

A match-arm reference resolves to the binding introduced by its pattern.

```tspp main.tspp
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ definition:match_left
                     ^^^^ reference:match_left
};
```

```query goto_definition main.tspp#reference:match_left
@goto_definition.target origin=main.tspp#reference:match_left location=main.tspp#definition:match_left symbol=main.tspp#left@2
```

## Labels

### Resolve a control label definition

A labeled break resolves to its enclosing label.

```tspp main.tspp
function choose(): int32 {
    outer: loop {
    ^ target:outer:start
    ^^^^^ definition:outer
        break outer: 1;
              ^^^^^ reference:outer
    }
    ^ target:outer:end
}
```

```query goto_definition main.tspp#reference:outer
@goto_definition.target origin=main.tspp#reference:outer location=main.tspp#target:outer selection=main.tspp#definition:outer symbol=main.tspp#outer@2
```

## Calls

### Resolve a tagged-template function definition

A tagged template resolves its tag like an ordinary call.

```tspp main.tspp
function sql(parts: string[], ...values: int32[]): string {
^ declaration:sql:start
         ^^^ definition:sql
    return "";
}
^ declaration:sql:end

const query = sql`select ${1}`;
              ^^^ reference:sql
```

```query goto_definition main.tspp#reference:sql
@goto_definition.target origin=main.tspp#reference:sql location=main.tspp#declaration:sql selection=main.tspp#definition:sql symbol=main.tspp#sql@1
```

### Resolve a const call definition

A const call resolves to the called function.

```tspp main.tspp
function build(): int32 {
^ declaration:build:start
         ^^^^^ definition:build
    return 1;
}
^ declaration:build:end

const value = const build();
                    ^^^^^ reference:build
```

```query goto_definition main.tspp#reference:build
@goto_definition.target origin=main.tspp#reference:build location=main.tspp#declaration:build selection=main.tspp#definition:build symbol=main.tspp#build@1
```

## Using Bindings

### Resolve a using binding definition

A using reference resolves to its lexical binding.

```tspp main.tspp
declare function openSession(): Dispose;

using session = openSession();
      ^^^^^^^ definition:session

const value = session;
              ^^^^^^^ reference:session
```

```query goto_definition main.tspp#reference:session
@goto_definition.target origin=main.tspp#reference:session location=main.tspp#definition:session symbol=main.tspp#session@2
```

## Annotations

### Resolve an annotation definition

An annotation application resolves through ordinary value identity.

```tspp main.tspp
newtype tracked = ();
^ declaration:tracked:start
        ^^^^^^^ definition:tracked
                   ^ declaration:tracked:end

@tracked
 ^^^^^^^ reference:tracked
class Service {}
```

```query goto_definition main.tspp#reference:tracked
@goto_definition.target origin=main.tspp#reference:tracked location=main.tspp#declaration:tracked selection=main.tspp#definition:tracked symbol=main.tspp#tracked@1
```
