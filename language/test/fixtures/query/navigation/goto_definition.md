# Goto Definition

## Local Variables

### Local variable definition

Goto definition on a variable reference should jump to its declaration.

```ds
const foo = 1;
//      ^^^ def:foo

const bar = foo;
//            ^^^ use:foo
```

The `foo` in `const bar = foo` references the `foo` defined above, so goto definition should navigate from `use:foo` to `def:foo`.

```query goto_definition use:foo
def:foo
```

### Function parameter definition

Goto definition on a parameter reference should jump to the parameter declaration.

```ds
function add(x: int32, y: int32): int32 {
//             ^ def:x
    return x + y;
//           ^ use:x
}
```

The `x` in `return x + y` references the parameter `x` in the function signature, so goto definition should navigate from `use:x` to `def:x`.

```query goto_definition use:x
def:x
```

## Functions

### Function definition

Goto definition on a function call should jump to the function declaration.

```ds
function greet(name: string): string {
//         ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//            ^^^^^ use:greet
```

`greet("World")` calls the `greet` function defined above, so goto definition should navigate from `use:greet` to `def:greet`.

```query goto_definition use:greet
def:greet
```

## Struct Fields

### Goto definition on struct field access

Goto definition on a field access should jump to the field declaration.

```ds
struct Point {
    x: int32
//    ^ def:point_x
    y: int32
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
//                     ^ use:point_x
}
```

Goto definition from `p.x` should navigate to the `x` field in `Point`.

```query goto_definition use:point_x
def:point_x
```

## Class Methods

### Goto definition on class method call

Goto definition on a class method call should jump to the method declaration.

```ds
class Logger {
    log(message: string): void {
//    ^^^ def:logger_log
        print(message);
    }
}

function main() {
    const logger = new Logger();
    logger.log("hello");
//           ^^^ use:logger_log
}
```

Goto definition from `logger.log` should navigate to the method definition.

```query goto_definition use:logger_log
def:logger_log
```

## Extension Methods

### Goto definition on extension method call

Goto definition on an extension method call should resolve to the extension method definition.

```ds
struct Calculator {}

extension of Calculator {
    add(x: int32, y: int32): int32 {
//    ^^^ def:calc_add
        return x + y;
    }
}

function main() {
    const calc = Calculator {};
    const result = calc.add(1, 2);
//                        ^^^ use:calc_add
}
```

Goto definition from `calc.add` should navigate to the extension method definition.

```query goto_definition use:calc_add
def:calc_add
```

## Enum Members

### Goto definition on enum member access

Goto definition on an enum member access should jump to the enum member declaration.

```ds
enum Status {
    Pending,
//    ^^^^^^^ def:pending
    Active,
}

const current = Status.Pending;
//                       ^^^^^^^ use:pending
```

```query goto_definition use:pending
def:pending
```

## Unresolved Symbols

### Goto definition on unresolved identifiers

Goto definition should return no result for unresolved identifiers.

```ds
function main(): void {
    missingValue;
//    ^^^^^^^^^^^^ unresolved
}
```

```query goto_definition unresolved
<none>
```

## Damaged Syntax

### Keep definitions working in damaged files

Goto definition should still work for valid symbols in files with neighboring malformed syntax.

```ds
const value = 1;
//      ^^^^^ def:value

function main(): void {
    value;
//    ^^^^^ use:value
    missingValue.
}
```

```query goto_definition use:value
def:value
```

### Keep definitions working after malformed function declarations

Goto definition should still work for later declarations after one malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query goto_definition use:stableLater
def:stableLater
```

### Keep definitions working after malformed call statements

Goto definition should still work for later declarations after one malformed call statement.

```ds
broken(,

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query goto_definition use:stableLater
def:stableLater
```

### Keep definitions working after bare new recovery statements

Goto definition should still work for later declarations after one bare `new` recovery statement.

```ds
new

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query goto_definition use:stableLater
def:stableLater
```

### Keep definitions working after throw recovery statements

Goto definition should still work for later declarations after one recovered `throw` statement.

```ds
throw

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query goto_definition use:stableLater
def:stableLater
```

### Keep definitions working after yield star recovery statements

Goto definition should still work for later declarations after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

export function stableLater(): void {}
//                ^^^^^^^^^^^ def:stableLater

stableLater();
//^^^^^^^^^^^ use:stableLater
```

```query goto_definition use:stableLater
def:stableLater
```

## Type And Value Imports

### Resolve type-only and value imports in one module

Goto definition should resolve type imports and value imports independently in the same file.

```ds:types.ds
export type Options = {
//            ^^^^^^^ def:type_options
    enabled: boolean,
};
```

```ds:values.ds
export const optionsValue = 1;
//             ^^^^^^^^^^^^ def:value_options
```

```ds:main.ds
import type { Options } from "./types.ds";
//              ^^^^^^^ use:type_options
import { optionsValue } from "./values.ds";

const typed: Options = { enabled: true };
const value = optionsValue;
//              ^^^^^^^^^^^^ use:value_options
```

```query goto_definition use:type_options
def:type_options
```

```query goto_definition use:value_options
def:value_options
```

### Resolve definition through aliased imports

Goto definition on an import alias usage should navigate to the exported source declaration.

```ds:alias_lib.ds
export function greetAliasSource(name: string): string {
//                ^^^^^^^^^^^^^^^^ def:greet_alias_source
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greetAliasSource as localGreeting } from "./alias_lib.ds";

const message = localGreeting("Destack");
//                ^^^^^^^^^^^^^ use:local_greeting
```

```query goto_definition use:local_greeting
def:greet_alias_source
```

## Damaged Syntax

### Return no definition for malformed member access

Goto definition should fail gracefully when the source is syntactically damaged at the cursor.

```ds
function main(): void {
    value.
//    ^^^^^ broken
}
```

```query goto_definition broken
<none>
```

## Associated Types

### Goto definition on associated type projection

Goto definition should resolve associated type projections to the associated type declaration.

```ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
//         ^^^^^ def:assoc_label
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
//                                    ^^^^^ use:assoc_label
```

```query goto_definition use:assoc_label
def:assoc_label
```

## Overloads

### Goto definition picks matching overload by argument shape

Goto definition should pick the matching overload target at each call site.

```ds
function parse(value: int32): int32 {
//         ^^^^^ def:parse_int
    return value;
}

function parse(value: string): string {
//         ^^^^^ def:parse_string
    return value;
}

const intValue = parse(1);
//                 ^^^^^ use:parse_int
const textValue = parse("ok");
//                  ^^^^^ use:parse_string
```

```query goto_definition use:parse_int
def:parse_int
```

```query goto_definition use:parse_string
def:parse_string
```

## Imports And Exports

### Goto definition for import aliases

Goto definition should resolve Destack import alias bindings.

```ds:main.ds
namespace bar {
    export const baz = 1;
//                 ^^^ def:import_alias_target
}

import Foo = bar.baz;

const value = Foo;
//              ^^^ use:import_alias_target
```

```query goto_definition use:import_alias_target
def:import_alias_target
```

### Goto definition through export namespace re exports

Goto definition should resolve through export namespace re export chains.

```ds:base.ds
export function ping(): void {}
//                ^^^^ def:ns_export_target
```

```ds:barrel.ds
export * as api from "./base.ds";
```

```ds:main.ds
import { api } from "./barrel.ds";

api.ping();
//    ^^^^ use:ns_export_target
```

```query goto_definition use:ns_export_target
def:ns_export_target
```

### Goto definition through default re-export alias chains

Goto definition should resolve through default re-export alias chains.

```ds:lib.ds
export default function buildWidget(): int32 {
//                        ^^^^^^^^^^^ def:buildWidget
    return 1;
}
```

```ds:barrel.ds
export { default as buildWidget } from "./lib.ds";
```

```ds:main.ds
import { buildWidget } from "./barrel.ds";

const value = buildWidget();
//              ^^^^^^^^^^^ use:buildWidget
```

```query goto_definition use:buildWidget
def:buildWidget
```
