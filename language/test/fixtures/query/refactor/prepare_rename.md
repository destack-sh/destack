# Prepare Rename

## Local Variables

### Prepare rename local variable

Prepare rename should return the current name as placeholder for local variables.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo + 2;
//          ^^^ ref:foo
```

When preparing to rename `foo` at its definition, we should get `foo` as the placeholder.

```query prepare_rename def:foo
foo
```

### Prepare rename at reference

We can also prepare rename from a reference site.

```ds
const value = 42;
//    ^^^^^ def:value

const result = value * 2;
//             ^^^^^ ref:value
```

Preparing rename at a reference should also return the current name.

```query prepare_rename ref:value
value
```

## Non-Renameable

### Prepare rename on literal

Prepare rename should fail on literals.

```ds
const value = 42;
//            ^^ literal
```

Preparing rename on a literal should return no result.

```query prepare_rename literal
<none>
```

## Members

### Prepare rename on struct field access

Prepare rename should resolve struct field accesses.

```ds
struct Point {
    x: int32
    y: int32
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
//                  ^ use:point_x
}
```

Preparing rename at `p.x` should return `x`.

```query prepare_rename use:point_x
x
```

### Prepare rename on method call

Prepare rename should resolve method calls.

```ds
class Counter {
    value: int32
    inc(): int32 {
        return this.value + 1;
    }
}

function main() {
    const counter = new Counter();
    const next = counter.inc();
//                       ^^^ use:counter_inc
}
```

Preparing rename at `counter.inc` should return `inc`.

```query prepare_rename use:counter_inc
inc
```

### Prepare rename on member definition

Prepare rename should resolve member definitions.

```ds
class Task {
    title: string;
//   ^^^^^ def:title
    done(): bool {
//  ^^^^ def:done
        return false;
    }
}
```

```query prepare_rename def:title
title
```

```query prepare_rename def:done
done
```

### Prepare rename on enum member

Prepare rename should resolve enum member references.

```ds
enum Status {
    Pending,
    Active,
}

const state = Status.Pending;
//                     ^ use:pending
```

```query prepare_rename use:pending
Pending
```

## Imports

### Prepare rename on type-only import usage

Prepare rename should work for type-only imports.

```ds:types.ds
export type Options = {
//          ^^^^^^^ def:Options
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(options: Options): void {
//                          ^^^^^^^ use:Options
    console.log(options.name);
}
```

Preparing rename at a type-only import usage should return the imported type name.

```query prepare_rename use:Options
Options
```

### Prepare rename on default import usage

Prepare rename should work for default imports.

```ds:default_export.ds
export default function greetDefault(name: string): string {
    return "Hello, " + name;
}
```

```ds:default_main.ds
import greetDefault from "./default_export.ds";

const message = greetDefault("Destack");
//              ^^^^^^^^^^^^ use:default_greet
```

```query prepare_rename use:default_greet
greetDefault
```

### Prepare rename on aliased import usage

Prepare rename should resolve local import aliases.

```ds:alias_export.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greet as localGreet } from "./alias_export.ds";

const message = localGreet("Destack");
//              ^^^^^^^^^^ use:local_greet_alias
```

```query prepare_rename use:local_greet_alias
localGreet
```

### Prepare rename on imported name in aliased import

Prepare rename on the imported name should resolve to the exported symbol name.

```ds:alias_prepare_lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_prepare_main.ds
import { greet as localGreet } from "./alias_prepare_lib.ds";
//       ^^^^^ use:greet_import_name

const message = localGreet("Destack");
```

```query prepare_rename use:greet_import_name
greet
```

### Prepare rename on namespace import alias usage

Prepare rename on a namespace import alias usage should resolve to the local alias name.

```ds:namespace_prepare_lib.ds
export function ping(): void {}
```

```ds:namespace_prepare_main.ds
import * as api from "./namespace_prepare_lib.ds";

api.ping();
// ^^^ use:namespace_alias_use
```

```query prepare_rename use:namespace_alias_use
api
```

### Prepare rename on default import alias usage

Prepare rename on a default import usage should resolve to the local alias name.

```ds:default_prepare_lib.ds
export default function greetDefault(name: string): string {
    return "Hello, " + name;
}
```

```ds:default_prepare_main.ds
import welcome from "./default_prepare_lib.ds";

const message = welcome("Destack");
//              ^^^^^^^ use:default_alias_use
```

```query prepare_rename use:default_alias_use
welcome
```

## Exports

### Prepare rename on default export definition

Prepare rename should return the default export name when targeting the declaration.

```ds
export default function greet(name: string): string {
//                      ^^^^^ def:default_greet
    return "Hello, " + name;
}
```

```query prepare_rename def:default_greet
greet
```

## Type Parameters

### Prepare rename on type parameter

Prepare rename should return the type parameter name.

```ds
function wrap<T>(value: T): T {
//            ^ def:type_param
    return value;
}
```

```query prepare_rename def:type_param
T
```

## Destructuring

### Prepare rename on destructured binding

Prepare rename should return the binding name for destructured targets.

```ds
const config = { value: 1 };
const { value } = config;
//      ^^^^^ def:destructure
```

```query prepare_rename def:destructure
value
```

### Prepare rename on namespace member usage

Prepare rename should resolve namespace member call usages.

```ds
namespace Api {
    export function ping(): void {}
}

Api.ping();
//  ^^^^ use:ping
```

```query prepare_rename use:ping
ping
```

### Prepare rename on parenthesized namespace receiver

Prepare rename should resolve namespace receiver symbols in parenthesized member calls.

```ds
namespace Api {
    export function ping(): void {}
}

(Api).ping();
// ^^^ use:api_namespace
```

```query prepare_rename use:api_namespace
Api
```

## TypeScript++ Surface

### Prepare rename on tagged template tag usage

Prepare rename should resolve tagged template tag functions.

```ds
function sql(parts: string[], ...values: int32): string {
    return "";
}

const value = sql`select ${1}`;
//            ^^^ use:sql_tag
```

```query prepare_rename use:sql_tag
sql
```

### Prepare rename on decorator usage

Prepare rename should resolve decorator identifiers.

```ds
function tracked<T>(value: T): T {
    return value;
}

@tracked
// ^^^^^^^ use:tracked_decorator
class Service {}
```

```query prepare_rename use:tracked_decorator
tracked
```

### Prepare rename on parameter decorator usage

Prepare rename should resolve decorators attached to function parameters.

```ds
function trackUsage(target: unknown): void {
    target;
}

function greet(@trackUsage name: string): string {
//              ^^^^^^^^^^ use:track_usage_param
    return name;
}
```

```query prepare_rename use:track_usage_param
trackUsage
```

### Prepare rename on match arm binding definition

Prepare rename should resolve bindings introduced by match patterns.

```ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
//   ^^^^ def:match_left
};
```

```query prepare_rename def:match_left
left
```

### Prepare rename on template literal type parameter definition

Prepare rename should resolve type parameters used inside template literal spans.

```ds
type Route<T extends string> = `api:${T}`;
//         ^ def:route_param
```

```query prepare_rename def:route_param
T
```

### Prepare rename on using binding usage

Prepare rename should resolve symbols introduced by `using` bindings.

```ds
using session = 1;

const next = session;
//           ^^^^^^^ use:using_session
```

```query prepare_rename use:using_session
session
```

### Prepare rename on comptime call usage

Prepare rename should resolve function symbols used in comptime expressions.

```ds
function build(): int32 {
    return 1;
}

const value = comptime build();
//                      ^^^^^ use:comptime_build
```

```query prepare_rename use:comptime_build
build
```

## Keywords

### Prepare rename on keyword

Prepare rename should fail on keywords.

```ds
function main(): void {
    return 1;
//  ^^^^^^ keyword:return
}
```

Preparing rename on a keyword should return no result.

```query prepare_rename keyword:return
<none>
```

## Unresolved Symbols

### Prepare rename on unresolved identifiers

Prepare rename should return no result for unresolved identifiers.

```ds
function main(): void {
    missingValue;
//  ^^^^^^^^^^^ unresolved
}
```

```query prepare_rename unresolved
<none>
```

## Damaged Syntax

### Prepare rename on valid symbols in damaged files

Prepare rename should still work for valid symbols in files with neighboring malformed syntax.

```ds
function main(): void {
    const value = 1;
//        ^^^^^ target:value
    missingValue.
}
```

```query prepare_rename target:value
value
```

### Prepare rename after malformed function declarations

Prepare rename should still work for later declarations after one malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}
//              ^^^^^^^^^^^ target:stableLater

stableLater();
```

```query prepare_rename target:stableLater
stableLater
```

### Prepare rename after malformed call statements

Prepare rename should still work for later declarations after one malformed call statement.

```ds
broken(,

export function stableLater(): void {}
//              ^^^^^^^^^^^ target:stableLater

stableLater();
```

```query prepare_rename target:stableLater
stableLater
```

### Prepare rename after bare new recovery statements

Prepare rename should still work for later declarations after one bare `new` recovery statement.

```ds
new

export function stableLater(): void {}
//              ^^^^^^^^^^^ target:stableLater

stableLater();
```

```query prepare_rename target:stableLater
stableLater
```

### Prepare rename after throw recovery statements

Prepare rename should still work for later declarations after one recovered `throw` statement.

```ds
throw

export function stableLater(): void {}
//              ^^^^^^^^^^^ target:stableLater

stableLater();
```

```query prepare_rename target:stableLater
stableLater
```

### Prepare rename after yield star recovery statements

Prepare rename should still work for later declarations after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

export function stableLater(): void {}
//              ^^^^^^^^^^^ target:stableLater

stableLater();
```

```query prepare_rename target:stableLater
stableLater
```

### Prepare rename on malformed unresolved member access

Prepare rename should return no result when the cursor is on malformed unresolved syntax.

```ds
function main(): void {
    missingValue.
//  ^^^^^^^^^^^ broken
}
```

```query prepare_rename broken
<none>
```

## Associated Types

### Prepare rename on associated type projection usage

Prepare rename should resolve associated type projections.

```ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
//                                  ^^^^^ use:assoc_label
```

```query prepare_rename use:assoc_label
Label
```
