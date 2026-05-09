# Completion

## Basic Keywords

### Complete with keywords at statement position

At any statement position, completion should offer language keywords.

```ds
const foo = 1;
$0
```

```query completion $0
foo: variable
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Complete in empty file

Even in an empty file, completion should offer keywords.

```ds
$0
```

```query completion $0
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Avoid keywords in expression position

When completing inside an expression, do not include statement keywords.

```ds
function main() {
    const value = $0
}
```

```query completion $0
main: function
```

## Type Position

### Complete primitives after colon

In type annotation position (after `:`), should show primitive types but NOT keywords.

```ds
const x: $0
```

```query completion $0
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
```

### Complete after extends

After `extends` keyword, should show primitive types but NOT keywords.

```ds
class Child extends $0
```

```query completion $0
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
```

### Complete declared types in type position

User-defined types should appear in type position.

```ds
struct Point {
    x: int32
    y: int32
}

function test() {
    const p: $0 = Point { x: 0, y: 0 };
}
```

```query completion $0
Point: struct
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
test: function
```

### Prefer type-like completions in type position

Type positions should rank type-like candidates ahead of value-like matches with the same prefix.

```ds
struct PrintNode {}

function PrintValue(): void {}

function main() {
    const value: Pri$0 = PrintNode {};
}
```

```query completion $0
top: 2
[0] label=PrintNode kind=struct sort=10 sort_text=<none> detail=<none> edits=<none>
[1] label=PrintValue kind=function sort=10 sort_text=<none> detail=<none> edits=<none>
```

### Exclude pure value symbols in type position

Pure value symbols should not appear in type position completions.
Destack function declarations still participate in the type namespace.

```ds
const value = 1;

function test() {
    const v: $0
}
```

```query completion $0
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
test: function
```

### Complete imported types in type position

Imported type aliases should appear in type position.

```ds:types.ds
export type Options = {
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(config: $0/*type*/ Options) {}
```

```query completion $0
Options: type_parameter
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
configure: function
```

### Complete in generic type parameter

In generic brackets, should show type-space symbols.

```ds
struct Container<T> {
    value: T
}

struct Point {
    x: int32
    y: int32
}

function test() {
    const c: Container<$0> = Container {
        value: Point { x: 0, y: 0 }
    };
}
```

```query completion $0
Container: struct
Point: struct
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
test: function
```

### Complete declared types in unterminated annotations

Type completion should still resolve local declarations in incomplete files.

```ds
struct PendingType {}

function main() {
    const value: PendingT$0
```

```query completion $0
PendingType: struct
```

## Import Paths

### Suggest relative starters for empty import

When completing an empty import path, suggest relative path starters.

```ds
import { } from "$0"
```

```query completion $0
./: folder
../: folder
<root>: module
<test>: module
@destack/builtin: module
```

### Complete relative entries with prefix

Relative import completion should list matching folders and modules.

```ds:utils/helpers.ds
export function help(): void {}
```

```ds:user.ds
export function user(): void {}
```

```ds:main.ds
import { } from "./u$0"
```

```query completion $0
user: module
utils/: folder
```

### Complete relative entries in unterminated import path

Relative import completion should still work when the closing quote is missing.

```ds:utils/helpers.ds
export function help(): void {}
```

```ds:user.ds
export function user(): void {}
```

```ds:main.ds
import { } from "./u$0
```

```query completion $0
user: module
utils/: folder
```

### Complete entries inside a folder

Relative import completion should list entries within a subfolder.

```ds:utils/format.ds
export function format(): void {}
```

```ds:utils/format_more.ds
export function formatMore(): void {}
```

```ds:utils/forms/form.ds
export function form(): void {}
```

```ds:main.ds
import { } from "./utils/f$0"
```

```query completion $0
format: module
format_more: module
forms/: folder
```

## Import Clause

### Complete named imports from target module

Completion should list exported symbols from the referenced module.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import { $0 } from "./types.ds";
```

```query completion $0
Widget: struct
makeWidget: function
```

### Skip already imported items

Completion should not suggest items that are already imported in the same clause.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import { Widget, $0 } from "./types.ds";
```

```query completion $0
makeWidget: function
```

### Prefer type only symbols in type only imports

Type only import clauses should only include type space symbols.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import type { $0 } from "./types.ds";
```

```query completion $0
Widget: struct
```

## Member Access

### Complete fields and extension methods

Member access should include fields and extension methods of nominal types.

```ds
struct Point {
    x: int32
    y: int32
}

extension of Point {
    magnitude(): float32 {
        return 0.0;
    }
}

function main() {
    const p = Point { x: 1, y: 2 };
    p.$0
}
```

```query completion $0
x: field
y: field
magnitude: method
```

## Object Literals

### Complete missing fields from expected type

Object literal completion should suggest missing fields from the expected type.

```ds
type Config = {
    timeout: int32,
    retries: int32,
};

const cfg: Config = {
    timeout: 10,
    $0
};
```

```query completion $0
retries: field
```

### Complete in missing object literal value

Object literal value completion should still work when the value expression is missing.

```ds
type Config = {
    user: string,
};

function main() {
    const userName = "Alice";
    const cfg: Config = {
        user: $0
    };
}
```

```query completion $0
userName: variable
main: function
```

## New Expressions

### Complete constructable symbols after new

Completion after `new` should only include constructable symbols.

```ds
class Alpha {}
struct Beta {}

function main() {
    const value = new $0
}
```

```query completion $0
Beta: struct
Alpha: class
```

## Auto Imports

### Auto import completions for missing symbols

Auto import completions should appear for missing symbols in value position.

```ds:lib.ds
export function makeWidget(): int32 {
    return 1;
}
```

```ds:main.ds
function main() {
    const value = make$0
}
```

```query completion $0
makeWidget: function
```

### Respect import type

Type-only imports should only suggest type symbols.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import type { $0 } from "./types.ds";
```

```query completion $0
Widget: struct
```

### Mixed type and value items

Each import item should use its own type or value filter.

```ds:types.ds
export type Options = {
    name: string,
};

export function makeOptions(): Options {
    return { name: "ok" };
}
```

```ds:main.ds
import { type $0, makeOptions } from "./types.ds";
import { type Options, ma$1 } from "./types.ds";
```

```query completion $0
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

```query completion $1
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Default import with named clause

Default imports should not block named import completions.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import DefaultThing, { $0 } from "./types.ds";
```

```query completion $0
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Multiline empty named clause

Multiline import clauses should still resolve completions.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import {
    $0
} from "./types.ds";
```

```query completion $0
Widget: struct
makeWidget: function
```

### Missing import clause close before from

Named import completions should still work when the closing `}` is omitted before `from`.

```ds:types.ds
export struct Widget {
    value: int32
}

export function makeWidget(): Widget {
    return Widget { value: 1 };
}
```

```ds:main.ds
import { Widget, $0 from "./types.ds";
```

```query completion $0
makeWidget: function
```

## Member Access

### Complete struct name reference

In value position, struct names should be available as completions.

```ds
struct Point {
    x: int32
    y: int32
}

function main() {
    $0
}
```

```query completion $0
main: function
Point: struct
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Complete after dot

When triggering completion immediately after a dot, show all members.

```ds
struct Point2 {
    x: int32
    y: int32
}

function main2() {
    const p: Point2 = Point2 { x: 1, y: 2 };
    p.$0
}
```

```query completion $0
x: field
y: field
```

### Complete after optional chain dot

When triggering completion immediately after `?.`, show all members for the receiver type.

```ds
struct Point3 {
    x: int32
    y: int32
}

function main3() {
    const p: Point3 = Point3 { x: 1, y: 2 };
    p?.$0
}
```

```query completion $0
x: field
y: field
```

### Complete partial member name

When typing a partial member name after a dot, show matching members.

```ds
struct Point {
    xa: int32
    xb: int32
    y: int32
}

function main() {
    const p: Point = Point { xa: 1, xb: 2, y: 3 };
    p.x$0
}
```

```query completion $0
xa: field
xb: field
```

### Complete class methods

Class methods should appear after dot.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
        return a + b;
    }

    multiply(a: int32, b: int32): int32 {
        return a * b;
    }
}

function main() {
    const calc = new Calculator();
    calc.$0
}
```

```query completion $0
add: method
multiply: method
```

### Complete extension methods

Extension methods should appear after dot on the target type.

```ds
struct Calculator2 {}

extension of Calculator2 {
    sum(a: int32, b: int32): int32 {
        return a + b;
    }
}

function main() {
    const calc = Calculator2 {};
    calc.$0
}
```

```query completion $0
sum: method
```

## New Expressions

### Complete after new

After `new`, suggest constructable types.

```ds
class Engine {}
struct Wheel {
    size: int32
}
function makeWheel(): Wheel {
    return Wheel { size: 32 };
}

function main() {
    new $0
}
```

```query completion $0
Wheel: struct
Engine: class
```

### Auto import constructable types after new

Auto imports should include constructable types in `new` expressions.

```ds:lib.ds
export class Engine {}
```

```ds:main.ds
function main() {
    new Eng$0
}
```

```query completion $0
top: 1
[0] label=Engine kind=class sort=100 sort_text=1:0:0:0000:0000:0005:./lib:Engine detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import { Engine } from \"./lib\";"
```

## Object Literal

### Complete fields in empty object literal

When inside an empty object literal, suggest all fields.

```ds
struct Point {
    x: int32
    y: int32
}

function main() {
    const p: Point = Point { $0 };
}
```

```query completion $0
x: field
y: field
main: function
Point: struct
```

### Complete remaining fields in object literal

When inside an object literal with a known type, suggest remaining fields.

```ds
struct Config {
    host: string
    port: int32
    timeout: int32
}

function main() {
    const cfg: Config = Config { host: "localhost", $0 };
}
```

```query completion $0
port: field
timeout: field
main: function
Config: struct
```

### Avoid duplicate field and variable completions

Expected type fields should not be duplicated by same named locals.

```ds
struct Config {
    host: string
    timeout: int32
}

function main() {
    const timeout = 5;
    const cfg: Config = Config { $0 };
}
```

```query completion $0
host: field
timeout: field
main: function
Config: struct
```

### Complete shorthand values in untyped object literal

When no expected type is known, object literal completion should offer visible value symbols.

```ds
function main() {
    const host = "localhost";
    const port = 8080;
    const cfg = { $0 };
}
```

```query completion $0
host: variable
port: variable
main: function
```

### Avoid field completions in object value position

When typing a value inside an object literal field, completion should be value-based.

```ds
struct Config {
    host: string
    port: int32
    timeout: int32
}

function main() {
    const host = "localhost";
    const cfg: Config = Config { host: ho$0 };
}
```

```query completion $0
host: variable
```

### Complete fields in nested object literal

Nested object literals should suggest fields of the nested type.

```ds
struct Address {
    street: string
    city: string
}

struct Person {
    name: string
    address: Address
}

function main() {
    const person: Person = Person {
        name: "John",
        address: Address { $0 },
    };
}
```

```query completion $0
street: field
city: field
main: function
Address: struct
Person: struct
```

## Scope and Variables

### Complete local variables

Local variables should be available in completions.

```ds
function test() {
    const name = "hello";
    const count = 42;
    $0
}
```

```query completion $0
name: variable
count: variable
test: function
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Exclude variables declared later in the same scope

Completion should not show variables declared after the cursor.

```ds
function test() {
    $0
    const later = 1;
}
```

```query completion $0
test: function
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Complete variables from outer scope

Variables from enclosing scopes should be visible.

```ds
function outer() {
    const outerVar = 1;

    function inner() {
        const innerVar = 2;
        $0
    }
}
```

```query completion $0
innerVar: variable
outerVar: variable
inner: function
outer: function
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Complete function parameters

Function parameters should be available inside the function body.

```ds
function greet(name: string, count: int32) {
    $0
}
```

```query completion $0
name: variable
count: variable
greet: function
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

### Shadowed variables show inner binding

When a variable shadows another, the inner one should be preferred.

```ds
function test() {
    const x = "outer";
    {
        const x = 42;
        $0
    }
}
```

```query completion $0
x: variable
test: function
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

## Fuzzy Matching

### Fuzzy match with prefix

Exact prefix matches should have highest priority.

```ds
function toString() {}
function toNumber() {}
function fromString() {}

function main() {
    to$0
}
```

```query completion $0
toNumber: function
toString: function
goto: keyword
throw: keyword
typeof: keyword
function: keyword
extension: keyword
instanceof: keyword
constructor: keyword
```

### Fuzzy match case insensitive

Case-insensitive prefix should still match.

```ds
function ToString() {}
function ToNumber() {}

function main() {
    to$0
}
```

```query completion $0
ToNumber: function
ToString: function
goto: keyword
throw: keyword
typeof: keyword
function: keyword
extension: keyword
instanceof: keyword
constructor: keyword
```

### Fuzzy match camelCase boundaries

Typing initials should match camelCase symbols.

```ds
function getElementsByClassName() {}
function getElementById() {}
function querySelector() {}

function main() {
    geb$0
}
```

```query completion $0
getElementById: function
getElementsByClassName: function
```

### Fuzzy match substring

Characters appearing in order should match.

```ds
function completion() {}
function configuration() {}
function connection() {}

function main() {
    cmpl$0
}
```

```query completion $0
completion: function
```

## Enums

### Complete enum variants in type position

Enum names should appear in type position.

```ds
enum Color {
    Red,
    Green,
    Blue,
}

function test() {
    const c: $0 = Color.Red;
}
```

```query completion $0
Color: enum
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
test: function
```

### Complete enum members after dot

Enum members should appear after the enum name.

```ds
enum Status {
    Pending,
    Active,
    Completed,
}

function main() {
    const s = Status.$0
}
```

```query completion $0
Pending: enum_member
Active: enum_member
Completed: enum_member
```

## Function Arguments

### Complete in function argument position

In function call arguments, show value completions.

```ds
function greet(name: string) {}

function main() {
    const userName = "Alice";
    greet($0)
}
```

```query completion $0
userName: variable
greet: function
main: function
```

### Prefer the active parameter name in argument position

Argument completion should prefer the candidate whose name matches the active parameter.

```ds
function paint(count: int32, color: string): void {}

function main() {
    const count = 1;
    const color = "red";
    paint(count, co$0)
}
```

```query completion $0
color: variable
count: variable
```

### Prefer callable values for callable parameters

Argument completion should prefer callable candidates when the active parameter expects one callable value.

```ds
function invoke(callback: (message: string) -> void): void {}

function handler(message: string): void {}

function main() {
    const hash = 1;
    invoke(ha$0)
}
```

```query completion $0
handler: function
hash: variable
```

### Prefer exact nominal type matches for parameters

Argument completion should prefer candidates whose value type matches the active parameter type.

```ds
class Person {}
class Usage {}

function takes(user: Person): void {}

function main() {
    const userOne: Usage = Usage {};
    const userTwo: Person = Person {};
    takes(user$0)
}
```

```query completion $0
userTwo: variable
userOne: variable
```

### Prefer alias-backed type matches for parameters

Argument completion should keep direct alias matches ahead of candidates that only match through the alias target.

```ds
class Person {}
class Usage {}

type UserParam = Person;

function takes(user: UserParam): void {}

function main() {
    const userOne: UserParam = Person {};
    const userTwo: Person = Person {};
    const userThree: Usage = Usage {};
    takes(user$0)
}
```

```query completion $0
userOne: variable
userTwo: variable
userThree: variable
```

### Prefer union-member type matches for parameters

Argument completion should keep candidates whose value type is part of the expected union ahead of unrelated values.

```ds
class Person {}
class Usage {}
class Audit {}

function takes(user: Person | Usage): void {}

function main() {
    const userAudit: Audit = Audit {};
    const userPerson: Person = Person {};
    const userUsage: Usage = Usage {};
    takes(user$0)
}
```

```query completion $0
userUsage: variable
userPerson: variable
userAudit: variable
```

## Damaged Source

### Complete in missing initializer position

Value completion should still work in a missing declarator initializer slot.

```ds
function main() {
    const userName = "Alice";
    const value = $0
}
```

```query completion $0
userName: variable
main: function
```

### Exclude destructured bindings in missing initializer position

Completion should exclude bindings that are still being introduced by one destructuring initializer.

```ds
function main() {
    const sourceData = {
        userName: "Alice",
        profile: "admin",
    };

    const { userName, profile: profileAlias } = $0
}
```

```query completion $0
sourceData: variable
main: function
```

### Keep member completions when one initializer introduces the same label

Initializer binding exclusion should only suppress lexical value names, not member completions.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
        return a + b;
    }
}

function main() {
    const calc = new Calculator();
    const add = calc.ad$0
}
```

```query completion $0
add: method
```

### Complete after malformed call statements

Value completion should still work for later call arguments after one malformed call statement.

```ds
broken(,

function greet(name: string): void {}

const userName = "Alice";
greet($0)
```

```query completion $0
userName: variable
greet: function
```

### Complete after bare new recovery statements

Value completion should still work for later call arguments after one bare `new` recovery statement.

```ds
function greet(name: string): void {}

function main() {
    new
    const userName = "Alice";
    greet($0)
}
```

```query completion $0
userName: variable
greet: function
main: function
```

### Complete after throw recovery statements

Value completion should still work for later call arguments after one recovered `throw` statement.

```ds
function greet(name: string): void {}

function main() {
    throw
    const userName = "Alice";
    greet($0)
}
```

```query completion $0
userName: variable
greet: function
main: function
```

### Complete after yield star recovery statements

Value completion should still work for later call arguments after one recovered `yield*` statement.

```ds
function greet(name: string): void {}

function* main() {
    yield*
    const userName = "Alice";
    greet($0)
}
```

```query completion $0
userName: variable
greet: function
main: function
```

### Complete in missing return position

Value completion should still work in a missing return value slot.

```ds
function helper(): void {}

function main() {
    const userName = "Alice";
    return $0
}
```

```query completion $0
userName: variable
helper: function
main: function
```

### Complete in missing yield position

Value completion should still work in a missing yield value slot.

```ds
function* main() {
    const userName = "Alice";
    yield $0
}
```

```query completion $0
userName: variable
main: function
```

### Complete in unterminated function argument position

In function call arguments, completion should still work when the closing `)` is missing.

```ds
function greet(name: string, suffix: string) {}

function main() {
    const userName = "Alice";
    greet(userName, $0
}
```

```query completion $0
userName: variable
greet: function
main: function
```

### Complete after malformed function declaration

Completion should still see later declarations after a malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}

function main() {
    const result = stable$0
}
```

```query completion $0
stableLater: function
```

## Interface Members

### Complete interface method implementations

When implementing an interface, suggest required members.

```ds
interface Drawable {
    function draw(): void;
    function getArea(): float64;
}

class Circle implements Drawable {
    $0
}
```

```query completion $0
Circle: class
as: keyword
do: keyword
if: keyword
in: keyword
is: keyword
of: keyword
any: keyword
for: keyword
get: keyword
let: keyword
new: keyword
set: keyword
try: keyword
case: keyword
else: keyword
enum: keyword
from: keyword
goto: keyword
loop: keyword
move: keyword
null: keyword
self: keyword
this: keyword
true: keyword
type: keyword
with: keyword
async: keyword
await: keyword
break: keyword
catch: keyword
class: keyword
const: keyword
false: keyword
final: keyword
infer: keyword
keyof: keyword
match: keyword
never: keyword
super: keyword
throw: keyword
union: keyword
using: keyword
where: keyword
while: keyword
yield: keyword
assert: keyword
export: keyword
import: keyword
public: keyword
return: keyword
static: keyword
struct: keyword
switch: keyword
typeof: keyword
asserts: keyword
declare: keyword
default: keyword
extends: keyword
finally: keyword
newtype: keyword
package: keyword
private: keyword
abstract: keyword
accessor: keyword
comptime: keyword
continue: keyword
debugger: keyword
function: keyword
override: keyword
provides: keyword
readonly: keyword
extension: keyword
interface: keyword
namespace: keyword
protected: keyword
satisfies: keyword
undefined: keyword
implements: keyword
instanceof: keyword
constructor: keyword
```

## Newtypes

### Complete newtype name in type position

Newtype names should appear in type position like other nominal types.

```ds
newtype UserId = int64;
newtype OrderId = int64;

function test() {
    const id: $0 = UserId(1);
}
```

```query completion $0
UserId: type_parameter
OrderId: type_parameter
any: type_parameter
int: type_parameter
int8: type_parameter
null: type_parameter
uint: type_parameter
void: type_parameter
float: type_parameter
int16: type_parameter
int32: type_parameter
int64: type_parameter
isize: type_parameter
never: type_parameter
uint8: type_parameter
usize: type_parameter
bigint: type_parameter
int128: type_parameter
int256: type_parameter
number: type_parameter
object: type_parameter
string: type_parameter
symbol: type_parameter
uint16: type_parameter
uint32: type_parameter
uint64: type_parameter
boolean: type_parameter
float32: type_parameter
float64: type_parameter
uint128: type_parameter
uint256: type_parameter
unknown: type_parameter
character: type_parameter
undefined: type_parameter
unique symbol: type_parameter
test: function
```

## Generics

### Complete generic struct fields

Generic struct instantiations should complete their fields.

```ds
struct Container<T> {
    value: T
    count: int32
}

function main() {
    const c: Container<string> = Container { value: "hello", count: 1 };
    c.$0
}
```

```query completion $0
value: field
count: field
```

## Auto Imports

### Suggest auto imports after prefix

Auto imports should appear after local symbols and match the prefix.

```ds:lib.ds
export function formatName(value: string): string {
    return value;
}
```

```ds:main.ds
function formatLocal(value: string): string {
    return value;
}

function main() {
    fo$0
}
```

```query completion $0
top: 2
[0] label=formatLocal kind=function sort=10 sort_text=<none> detail=<none> edits=<none>
[1] label=formatName kind=function sort=100 sort_text=1:0:0:0000:0000:0005:./lib:formatName detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import { formatName } from \"./lib\";"
```

### Suggest auto imports on explicit invocation with short prefix

Explicit completion invocation should include auto imports even with a single character prefix.

```ds:lib.ds
export function zetaGreeting(): void {}
```

```ds:main.ds
function main() {
    z$0
}
```

```query completion $0
top: 1
[0] label=zetaGreeting kind=function sort=100 sort_text=1:0:0:0000:0000:0005:./lib:zetaGreeting detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import { zetaGreeting } from \"./lib\";"
```

### Prefer stronger case matches for auto imports

Auto imports should rank exact-case prefix matches ahead of case-folded matches.

```ds:http_server.ds
export function HTTPServer(): void {}
```

```ds:http_tool.ds
export function HttpServer(): void {}
```

```ds:main.ds
function main() {
    HT$0
}
```

```query completion $0
top: 2
[0] label=HTTPServer kind=function sort=100 sort_text=1:0:0:0000:0000:0013:./http_server:HTTPServer detail=Auto import from ./http_server edits=main.ds:1:1-1:1=>"import { HTTPServer } from \"./http_server\";"
[1] label=HttpServer kind=function sort=100 sort_text=1:0:0:0000:0000:0011:./http_tool:HttpServer detail=Auto import from ./http_tool edits=main.ds:1:1-1:1=>"import { HttpServer } from \"./http_tool\";"
```

### Prefer locals over auto imports with stronger matches

Local symbols should rank ahead of auto imports even when the auto import is a better text match.

```ds:lib.ds
export function color(): void {}
```

```ds:main.ds
function setColor(): void {}

function main() {
    co$0
}
```

```query completion $0
top: 2
[0] label=setColor kind=function sort=10 sort_text=<none> detail=<none> edits=<none>
[1] label=color kind=function sort=100 sort_text=1:0:0:0000:0000:0005:./lib:color detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import { color } from \"./lib\";"
```

### Prefer whole name matches over longer prefixes

Whole-name lexical matches should rank ahead of longer prefix matches, even when the whole match is case folded.

```ds
function Format(): void {}

function formatName(): void {}

function main() {
    format$0
}
```

```query completion $0
top: 2
[0] label=Format kind=function sort=10 sort_text=<none> detail=<none> edits=<none>
[1] label=formatName kind=function sort=10 sort_text=<none> detail=<none> edits=<none>
```

### Skip auto imports for visible names

Auto imports should not be suggested when a matching name is already visible.

```ds:lib.ds
export struct Widget {}
```

```ds:main.ds
function main() {
    const Widget = 1;
    Wid$0
}
```

```query completion $0
Widget: variable
```

### Use type only auto imports for pure type exports in type position

Auto imports in type position should use `import type` for pure type exports.

```ds:lib.ds
export type Widget = {
    value: string,
};
```

```ds:main.ds
type Alias = Wid$0;
```

```query completion $0
top: 1
[0] label=Widget kind=type_parameter sort=100 sort_text=0:0:0:0000:0000:0005:./lib:Widget detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import type { Widget } from \"./lib\";"
```

### Use type only auto imports for pure type exports in type position

Destack pure type exports should also use `import type` in type position.

```ds:lib.ds
export type Widget = {
    value: string,
};
```

```ds:main.ds
export {}

type Alias = Wid$0
```

```query completion $0
top: 1
[0] label=Widget kind=type_parameter sort=100 sort_text=0:0:0:0000:0000:0005:./lib:Widget detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import type { Widget } from \"./lib\";"
```

### Use type only auto imports for type-value exports in type position

Destack type-position auto imports should stay type-only even for dual-space exports.

```ds:lib.ds
export class WobbleWidget {}
```

```ds:main.ds
export {}

type Alias = Wob$0
```

```query completion $0
top: 1
[0] label=WobbleWidget kind=class sort=100 sort_text=1:0:0:0000:0000:0005:./lib:WobbleWidget detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import type { WobbleWidget } from \"./lib\";"
```

### Prefer pure type exports in type position

Type-position auto imports should rank pure type exports ahead of type-value exports with the same name.

```ds:type_lib.ds
export type Widget = {
    value: string,
};
```

```ds:class_lib.ds
export class Widget {}
```

```ds:main.ds
type Alias = Widg$0;
```

```query completion $0
top: 2
[0] label=Widget kind=type_parameter sort=100 sort_text=0:0:0:0000:0000:0010:./type_lib:Widget detail=Auto import from ./type_lib edits=main.ds:1:1-1:1=>"import type { Widget } from \"./type_lib\";"
[1] label=Widget kind=class sort=100 sort_text=1:0:0:0000:0000:0011:./class_lib:Widget detail=Auto import from ./class_lib edits=main.ds:1:1-1:1=>"import { Widget } from \"./class_lib\";"
```

### Use value imports for type-value exports in .ds type position

Destack type-value exports should stay full imports even when the reference is in type position.

```ds:lib.ds
export struct Widget {}
```

```ds:main.ds
type Alias = Wid$0;
```

```query completion $0
top: 1
[0] label=Widget kind=struct sort=100 sort_text=1:0:0:0000:0000:0005:./lib:Widget detail=Auto import from ./lib edits=main.ds:1:1-1:1=>"import { Widget } from \"./lib\";"
```

### Prefer same folder auto imports

Auto imports from the same folder should rank above nearby and far modules.

```ds:format_root.ds
export function formatSameRoot(value: string): string {
    return value;
}
```

```ds:near/format.ds
export function formatSameNearby(value: string): string {
    return value;
}
```

```ds:far/deeper/format.ds
export function formatSameFar(value: string): string {
    return value;
}
```

```ds:main.ds
function main() {
    formatSa$0
}
```

```query completion $0
top: 3
[0] label=formatSameRoot kind=function sort=100 sort_text=1:0:0:0000:0000:0013:./format_root:formatSameRoot detail=Auto import from ./format_root edits=main.ds:1:1-1:1=>"import { formatSameRoot } from \"./format_root\";"
[1] label=formatSameNearby kind=function sort=100 sort_text=1:1:0:0001:0001:0013:./near/format:formatSameNearby detail=Auto import from ./near/format edits=main.ds:1:1-1:1=>"import { formatSameNearby } from \"./near/format\";"
[2] label=formatSameFar kind=function sort=100 sort_text=1:1:0:0002:0002:0019:./far/deeper/format:formatSameFar detail=Auto import from ./far/deeper/format edits=main.ds:1:1-1:1=>"import { formatSameFar } from \"./far/deeper/format\";"
```

### Prefer same package auto imports over external packages

Auto imports from the current package should rank ahead of equally named external package exports.

```json:package.json
{ "name": "app" }
```

```ds:same_package.ds
export function formatWidget(): void {}
```

```json:node_modules/ext_pkg/package.json
{ "name": "ext_pkg" }
```

```ds:node_modules/ext_pkg/format_widget.ds
export function formatWidget(): void {}
```

```ds:main.ds
function main() {
    formatW$0
}
```

```query completion $0
top: 2
[0] label=formatWidget kind=function sort=100 sort_text=1:0:0:0000:0000:0014:./same_package:formatWidget detail=Auto import from ./same_package edits=main.ds:1:1-1:1=>"import { formatWidget } from \"./same_package\";"
[1] label=formatWidget kind=function sort=100 sort_text=1:1:2:0002:0002:0021:ext_pkg/format_widget:formatWidget detail=Auto import from ext_pkg/format_widget edits=main.ds:1:1-1:1=>"import { formatWidget } from \"ext_pkg/format_widget\";"
```

### Rank auto imports by proximity

Auto imports from closer paths should rank ahead of farther ones.

```ds:near/format.ds
export function formatNearby(value: string): string {
    return value;
}
```

```ds:far/deeper/format.ds
export function formatFar(value: string): string {
    return value;
}
```

```ds:main.ds
function main() {
    for$0
}
```

```query completion $0
top: 2
[0] label=formatNearby kind=function sort=100 sort_text=1:1:0:0001:0001:0013:./near/format:formatNearby detail=Auto import from ./near/format edits=main.ds:1:1-1:1=>"import { formatNearby } from \"./near/format\";"
[1] label=formatFar kind=function sort=100 sort_text=1:1:0:0002:0002:0019:./far/deeper/format:formatFar detail=Auto import from ./far/deeper/format edits=main.ds:1:1-1:1=>"import { formatFar } from \"./far/deeper/format\";"
```

### Tie break auto imports by path

When auto import scores tie, ordering should be stable by path.

```ds:a/format_tie.ds
export function formatTieAlpha(value: string): string {
    return value;
}
```

```ds:b/format_tie.ds
export function formatTieBeta(value: string): string {
    return value;
}
```

```ds:main.ds
function main() {
    formatTie$0
}
```

```query completion $0
top: 2
[0] label=formatTieAlpha kind=function sort=100 sort_text=1:1:0:0001:0001:0014:./a/format_tie:formatTieAlpha detail=Auto import from ./a/format_tie edits=main.ds:1:1-1:1=>"import { formatTieAlpha } from \"./a/format_tie\";"
[1] label=formatTieBeta kind=function sort=100 sort_text=1:1:0:0001:0001:0014:./b/format_tie:formatTieBeta detail=Auto import from ./b/format_tie edits=main.ds:1:1-1:1=>"import { formatTieBeta } from \"./b/format_tie\";"
```

### Complete constructor names after bare new

Recovered `new` constructor slots should complete constructor names from scope.

```ds
class Engine {}

function main() {
    new Eng$0
}
```

```query completion $0
Engine: class
```

### Complete values after return

Recovered `return` value slots should complete visible values from scope.

```ds
function helperValue(): int32 {
    return 1;
}

function main() {
    return hel$0
}
```

```query completion $0
helperValue: function
```

### Complete values after throw

Recovered `throw` value slots should complete visible values from scope.

```ds
function main() {
    const failureValue = 1;
    throw fai$0
}
```

```query completion $0
failureValue: variable
```

### Complete values after yield star

Recovered `yield*` operand slots should complete visible values from scope.

```ds
function* iteratorItems() {
    yield 1;
}

function* main() {
    yield* iteratorI$0
}
```

```query completion $0
iteratorItems: function
```
