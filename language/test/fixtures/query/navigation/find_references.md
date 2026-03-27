# Find References

## Local Variables

### Find references to local variable

Find references should return all occurrences of a symbol, including its definition.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo;
//          ^^^ use:foo
const baz = foo + foo;
```

`foo` is defined once and used 3 times (in `bar`, and twice in `baz`), so the total is 4 references.

```query find_references def:foo
main.ds:1:7-1:10
main.ds:3:13-3:16
main.ds:4:13-4:16
main.ds:4:19-4:22
```

## Functions

### Find references to function

Find references on a function should return its definition and all call sites.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const a = greet("World");
//        ^^^^^ use:greet
const b = greet("Test");
```

`greet` is defined once and called twice (in `a` and `b`), so the total is 3 references.

```query find_references def:greet
main.ds:1:10-1:15
main.ds:5:11-5:16
main.ds:6:11-6:16
```

## Methods

### Find references to class method

Find references should include the method definition and all call sites.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
//  ^^^ def:calc_add
        return a + b;
    }
}

function main() {
    const calc = new Calculator();
    const first = calc.add(1, 2);
//                     ^^^ use:calc_add_1
    const second = calc.add(3, 4);
//                      ^^^ use:calc_add_2
}
```

`add` appears 3 times: its definition and two calls.

```query find_references def:calc_add
main.ds:2:5-2:8
main.ds:9:24-9:27
main.ds:10:25-10:28
```

## Struct Fields

### Find references to a struct field

Find references should include the field definition and all field accesses.

```ds
struct Point {
    x: int32
//  ^ def:field_x
    y: int32
}

function main(p: Point) {
    const a = p.x;
//              ^ use:field_x_1
    const b = p.x + p.x;
//              ^ use:field_x_2
//                    ^ use:field_x_3
}
```

`x` appears 4 times for this symbol: its definition and three accesses.

```query find_references def:field_x
main.ds:2:5-2:6
main.ds:7:17-7:18
main.ds:8:17-8:18
main.ds:8:23-8:24
```

## Cross Module

### Find references across modules

Find references should include imports and call sites in other modules.

```ds:lib.ds
export function ping(): void {}
//              ^^^^ def:ping
```

```ds:main.ds
import { ping } from "./lib.ds";
//       ^^^^ use:ping_import

ping();
// ^^^^ use:ping_call
```

```query find_references def:ping
lib.ds:1:17-1:21
main.ds:1:10-1:14
main.ds:3:1-3:5
```

## Type-Only And Value Imports

### Find references across mixed import shapes

Find references should keep type-only imports and value imports in their own symbol graphs.

```ds:types.ds
export type Settings = {
//          ^^^^^^^^ def:settings_type
    enabled: boolean,
};
```

```ds:values.ds
export function settings(): int32 {
//              ^^^^^^^^ def:settings_value
    return 1;
}
```

```ds:main.ds
import type { Settings } from "./types.ds";
//            ^^^^^^^^ use:settings_type_import
import { settings } from "./values.ds";
//       ^^^^^^^^ use:settings_value_import

const typed: Settings = { enabled: true };
//          ^^^^^^^^ use:settings_type_use
const value = settings();
//            ^^^^^^^^ use:settings_value_call
```

```query find_references def:settings_type
types.ds:1:13-1:21
main.ds:1:15-1:23
main.ds:4:14-4:22
```

```query find_references def:settings_value
def:settings_value
use:settings_value_import
use:settings_value_call
```

## Re-Exports

### Find references through re-exported aliases

Find references should include re-exported aliases and downstream imports.

```ds:alias_base.ds
export function ping(): void {}
//              ^^^^ def:ping
```

```ds:alias_barrel.ds
export { ping as pingAlias } from "./alias_base.ds";
```

```ds:alias_main.ds
import { pingAlias } from "./alias_barrel.ds";

pingAlias();
```

```query find_references def:ping
alias_base.ds:1:17-1:21
alias_barrel.ds:1:10-1:14
alias_main.ds:1:10-1:19
alias_main.ds:3:1-3:10
```

## Enum Members

### Find references to enum members

Find references should include enum member declarations and member accesses.

```ds
enum Color {
    Red,
//  ^^^ def:red
    Blue,
}

const first = Color.Red;
//                   ^^^ use:red_1
const second = Color.Red;
//                    ^^^ use:red_2
```

```query find_references def:red
main.ds:2:5-2:8
main.ds:6:21-6:24
main.ds:7:22-7:25
```

## Namespace Receivers

### Find references for namespace receivers across member forms

Find references for a namespace symbol should include member receivers in plain and parenthesized forms.

```ds
namespace Api {
//        ^^^ def:api
    export function greet(): string {
        return "Hello";
    }
}

const one = Api.greet();
//          ^^^ use:api_1
const two = (Api).greet();
//           ^^^ use:api_2
```

```query find_references def:api
def:api
use:api_1
use:api_2
```

### Find references for import aliases

Find references on an import alias should stay scoped to the alias declaration and alias call sites.

```ds:alias_lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greet as localGreet } from "./alias_lib.ds";
//                ^^^^^^^^^^ def:local_greet_alias

const first = localGreet("Destack");
//            ^^^^^^^^^^ use:local_greet_alias
```

```query find_references use:local_greet_alias
def:local_greet_alias
use:local_greet_alias
```

### Find references for imported names in aliased imports

Find references on the imported side of an aliased import should follow the exported symbol graph.

```ds:alias_import_lib.ds
export function greet(name: string): string {
//              ^^^^^ def:greet_export
    return "Hello, " + name;
}
```

```ds:alias_import_main.ds
import { greet as localGreet } from "./alias_import_lib.ds";
//       ^^^^^ use:greet_import_name

const first = localGreet("Destack");
//            ^^^^^^^^^^ use:greet_alias_call
```

```query find_references use:greet_import_name
def:greet_export
use:greet_import_name
use:greet_alias_call
```

### Find references for namespace import aliases

Find references on a namespace import alias should stay in the local alias graph.

```ds:namespace_ref_lib.ds
export function ping(): void {}
```

```ds:namespace_ref_main.ds
import * as api from "./namespace_ref_lib.ds";
//          ^^^ def:namespace_import_alias

api.ping();
// ^^^ use:namespace_import_alias_1
(api).ping();
// ^^^ use:namespace_import_alias_2
```

```query find_references use:namespace_import_alias_1
namespace_ref_main.ds:1:13-1:16
namespace_ref_main.ds:3:1-3:4
namespace_ref_main.ds:4:2-4:5
```

### Find references for default import aliases

Find references on a default import alias should stay in the local alias graph.

```ds:default_ref_lib.ds
export default function greetDefault(): string {
//                      ^^^^^^^^^^^^ def:default_export_name
    return "hello";
}
```

```ds:default_ref_main.ds
import welcome from "./default_ref_lib.ds";
//     ^^^^^^^ def:default_import_alias

const first = welcome();
//            ^^^^^^^ use:default_import_alias_1
const second = welcome();
//             ^^^^^^^ use:default_import_alias_2
```

```query find_references use:default_import_alias_1
def:default_import_alias
use:default_import_alias_1
use:default_import_alias_2
```

### Ignore shadowed namespace receivers

Find references for a namespace receiver should ignore shadowed local bindings with the same name.

```ds
namespace Api {
//        ^^^ def:api_shadowed
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
//         ^^^ use:api_local_shadow
}

const message = Api.greet();
//              ^^^ use:api_namespace
```

```query find_references def:api_shadowed
def:api_shadowed
use:api_namespace
```

## TypeScript++ Surface

### Find references for tagged template tag functions

Find references on a tagged template tag should include the definition and tagged template call sites.

```ds
function sql(parts: string[], ...values: int32): string {
//       ^^^ def:sql_tag
    return "";
}

const first = sql`select ${1}`;
//            ^^^ use:sql_tag_1
const second = sql`where ${2}`;
//             ^^^ use:sql_tag_2
```

```query find_references def:sql_tag
def:sql_tag
use:sql_tag_1
use:sql_tag_2
```

### Find references for decorators

Find references on a decorator function should include decorator attachment sites.

```ds
function tracked<T>(value: T): T {
//       ^^^^^^^ def:tracked
    return value;
}

@tracked
//^^^^^^^ use:tracked_1
class Service {}

@tracked
//^^^^^^^ use:tracked_2
function greet(): void {}
```

```query find_references def:tracked
main.ds:1:10-1:17
main.ds:5:2-5:9
main.ds:8:2-8:9
```

### Find references for decorators on members and parameters

Find references on a decorator function should include member and parameter decorator sites.

```ds
function trackUsage(target: unknown): void {
//       ^^^^^^^^^^ def:track_usage
}

class User {
    @trackUsage
//   ^^^^^^^^^^ use:track_usage_member
    name: string = "";
}

function greet(@trackUsage name: string): string {
//              ^^^^^^^^^^ use:track_usage_param
    return name;
}
```

```query find_references def:track_usage
def:track_usage
use:track_usage_member
use:track_usage_param
```

### Find references for match arm bindings

Find references on a match arm binding should include the binding and in-arm uses.

```ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
//   ^^^^ def:match_left
//                  ^^^^ use:match_left
};
```

```query find_references def:match_left
main.ds:4:6-4:10
main.ds:4:22-4:26
```

### Find references for template literal type parameter uses

Find references on a template literal type parameter should include references inside template spans.

```ds
type Route<T extends string> = `api:${T}`;
//         ^ def:route_param
//                                 ^ use:route_param

type UsersRoute = Route<"users">;
```

```query find_references def:route_param
main.ds:1:12-1:13
main.ds:1:39-1:40
```

### Find references for using bindings

Find references on a `using` binding should include the binding and in scope usages.

```ds
function openSession(): int32 {
    return 1;
}

using session = openSession();
//    ^^^^^^^ def:using_session

const first = session;
//            ^^^^^^^ use:using_session_1
const second = session;
//             ^^^^^^^ use:using_session_2
```

```query find_references def:using_session
def:using_session
use:using_session_1
use:using_session_2
```

### Find references for comptime call targets

Find references on a function should include call sites in comptime expressions.

```ds
function scale(value: int32): int32 {
//       ^^^^^ def:comptime_scale
    return value * 2;
}

const a = comptime scale(2);
//                 ^^^^^ use:comptime_scale_1
const b = scale(3);
//        ^^^^^ use:comptime_scale_2
```

```query find_references def:comptime_scale
def:comptime_scale
use:comptime_scale_1
use:comptime_scale_2
```

## Damaged Syntax

### Find references for valid symbols after malformed function declarations

Find references should still resolve later declarations after one malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}
//              ^^^^^^^^^^^ def:stableLater

stableLater();
// ^^^^^^^^^^^ use:stableLater
```

```query find_references def:stableLater
main.ds:3:17-3:28
main.ds:5:1-5:12
```

### Find references for valid symbols after malformed call statements

Find references should still resolve later declarations after one malformed call statement.

```ds
broken(,

export function stableLater(): void {}
//              ^^^^^^^^^^^ def:stableLater

stableLater();
// ^^^^^^^^^^^ use:stableLater
```

```query find_references def:stableLater
main.ds:3:17-3:28
main.ds:5:1-5:12
```

### Find references for valid symbols after bare new recovery statements

Find references should still resolve later declarations after one bare `new` recovery statement.

```ds
new

export function stableLater(): void {}
//              ^^^^^^^^^^^ def:stableLater

stableLater();
// ^^^^^^^^^^^ use:stableLater
```

```query find_references def:stableLater
main.ds:3:17-3:28
main.ds:5:1-5:12
```

### Find references for valid symbols after throw recovery statements

Find references should still resolve later declarations after one recovered `throw` statement.

```ds
throw

export function stableLater(): void {}
//              ^^^^^^^^^^^ def:stableLater

stableLater();
// ^^^^^^^^^^^ use:stableLater
```

```query find_references def:stableLater
main.ds:3:17-3:28
main.ds:5:1-5:12
```

### Find references for valid symbols after yield star recovery statements

Find references should still resolve later declarations after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

export function stableLater(): void {}
//              ^^^^^^^^^^^ def:stableLater

stableLater();
// ^^^^^^^^^^^ use:stableLater
```

```query find_references def:stableLater
main.ds:6:17-6:28
main.ds:8:1-8:12
```

### Return no references for malformed unresolved access

Find references should fail gracefully when the cursor is on malformed unresolved syntax.

```ds
function main(): void {
    unknown.
//  ^^^^^^^ broken
}
```

```query find_references broken
<none>
```

## Associated Types

### Find references for associated type projections

Find references on an associated type should include projection use sites.

```ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
//       ^^^^^ def:assoc_label
}

class Message<T extends string> implements Envelope<T> {}

type EventLabel = Message<"orders">.Label<"created">;
//                                  ^^^^^ use:assoc_label
```

```query find_references def:assoc_label
def:assoc_label
use:assoc_label
```

## Dynamic Resolution

### Find references includes union receiver method calls

Find references should include calls through union receivers for shared members.

```ds
class Cat {
    speak(): string {
//  ^^^^^ def:cat_speak
        return "meow";
    }
}

class Dog {
    speak(): string {
//  ^^^^^ def:dog_speak
        return "woof";
    }
}

declare const pet: Cat | Dog;

const sound = pet.speak();
//                ^^^^^ use:pet_speak
```

```query find_references def:cat_speak
def:cat_speak
use:pet_speak
```

## Using

### Find references for await using bindings

Find references should include bindings introduced by await using.

```ds
async function run(): void {
    await using resource = 1;
//              ^^^^^^^^ def:await_using_resource

    const next = resource;
//               ^^^^^^^^ use:await_using_resource
}
```

```query find_references def:await_using_resource
def:await_using_resource
use:await_using_resource
```

### Find references for for using iteration bindings

Find references should include iteration bindings introduced by for using.

```ds
declare function values(): int32[];

for (using item of values()) {
//         ^^^^ def:for_using_item
    const next = item;
//               ^^^^ use:for_using_item
}
```

```query find_references def:for_using_item
def:for_using_item
use:for_using_item
```

### Find references for export using bindings

Find references should include export using declarations and local uses.

```ds
export using cache = 1;
//           ^^^^^ def:export_using_cache

const value = cache;
//            ^^^^^ use:export_using_cache
```

```query find_references def:export_using_cache
def:export_using_cache
use:export_using_cache
```

## Comptime

### Find references for comptime static parameters

Find references should include uses of comptime static parameters in type expressions.

```ds
type Buffer<comptime N: number> = uint8[N];
//                   ^ def:comptime_n
//                                    ^ use:comptime_n
```

```query find_references def:comptime_n
main.ds:1:22-1:23
main.ds:1:41-1:42
```

### Find references for comptime dynamic parameters

Find references should include uses of comptime dynamic parameters in function bodies.

```ds
function createBuffer(comptime size: int): int {
//                             ^^^^ def:comptime_size
    return size;
//         ^^^^ use:comptime_size
}
```

```query find_references def:comptime_size
def:comptime_size
use:comptime_size
```

## Decorators

### Find references for match arm decorators

Find references should include decorator usage on match arms.

```ds
function cold(target: unknown): void {
//       ^^^^ def:arm_decorator
    target;
}

declare const value: int32 | string;

const result = match (value) {
    @cold
//   ^^^^ use:arm_decorator
    0 => "zero"
    _ => "other"
};
```

```query find_references def:arm_decorator
def:arm_decorator
use:arm_decorator
```

### Find references for statement decorators

Find references should include decorator usage on statements.

```ds
function unroll(target: unknown): void {
//       ^^^^^^ def:statement_decorator
    target;
}

@unroll
// ^^^^^^ use:statement_decorator
for (let i = 0; i < 3; i++) {
    const _ = i;
}
```

```query find_references def:statement_decorator
main.ds:1:10-1:16
main.ds:5:2-5:8
```

## Imports And Exports

### Find references through default re-export alias chains

Find references should include default re-export aliases, downstream imports, and call sites.

```ds:lib.ds
export default function buildWidget(): int32 {
//                      ^^^^^^^^^^^ def:buildWidget
    return 1;
}
```

```ds:barrel.ds
export { default as buildWidget } from "./lib.ds";
//                  ^^^^^^^^^^^ use:buildWidget_reexport
```

```ds:main.ds
import { buildWidget } from "./barrel.ds";
//       ^^^^^^^^^^^ use:buildWidget_import

const value = buildWidget();
//            ^^^^^^^^^^^ use:buildWidget_call
```

```query find_references def:buildWidget
def:buildWidget
use:buildWidget_reexport
use:buildWidget_import
use:buildWidget_call
```

### Find references for namespace re-export aliases

Find references on a namespace re-export alias should stay in the local alias graph.

```ds:base.ds
export function ping(): void {}
```

```ds:barrel.ds
export * as api from "./base.ds";
//           ^^^ use:api_reexport
```

```ds:main.ds
import { api } from "./barrel.ds";
//       ^^^ use:api_import

api.ping();
// ^^^ use:api_call

const sameApi = api;
//              ^^^ use:api_value
```

```query find_references use:api_import
barrel.ds:1:13-1:16
main.ds:1:10-1:13
main.ds:3:1-3:4
main.ds:5:17-5:20
```
