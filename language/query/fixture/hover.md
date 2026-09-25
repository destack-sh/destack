
## Functions

### Hover over a function reference

Hover shows a declaration signature at its reference.

```tspp main.tspp
function greet(name: string): string {
         ^^^^^ definition
    return name;
}

const message = greet("Destack");
                ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function greet(name: string): string" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#reference
```

### Return the same declaration signature at its definition

Hovering a function name reports its declaration.

```tspp main.tspp
function greet(name: string): string {
         ^^^^^ definition
    return name;
}
```

```query hover main.tspp#definition
@hover.item index=0 declaration="function greet(name: string): string" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#definition
```

### Return the matching overload

A call reports the overload that accepts its arguments.

```tspp main.tspp
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const value = parse("one");
              ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function parse(value: string): string" location=main.tspp:5:1-7:2 selection=main.tspp:5:10-5:15 range=main.tspp#reference
```

### Return an overload family

A value reference to an overload family reports every declaration.

```tspp main.tspp
function parse(value: int32): int32 {
    return value;
}

function parse(value: string): string {
    return value;
}

const parser = parse;
               ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function parse(value: int32): int32" location=main.tspp:1:1-3:2 selection=main.tspp:1:10-1:15 range=main.tspp#reference
@hover.item index=1 declaration="function parse(value: string): string" location=main.tspp:5:1-7:2 selection=main.tspp:5:10-5:15 range=main.tspp#reference
```

### Include an applied generic instantiation

The generic declaration and applied callable type remain distinct.

```tspp main.tspp
function identity<Value>(value: Value): Value {
         ^^^^^^^^ definition
    return value;
}

declare const name: string;
const result = identity(name);
               ^^^^^^^^ reference
```

```query hover main.tspp#definition
@hover.item index=0 declaration="function identity<Value>(value: Value): Value" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#definition
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function identity<Value>(value: Value): Value" type="(value: string) => string" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#reference
```

### Preserve function hover across body changes

Changing a function body leaves its declaration hover unchanged.

```tspp main.tspp
function message(): string {
         ^^^^^^^ definition
    return "one";
}

const value = message();
              ^^^^^^^ reference

function unrelated(): int32 {
    return 1;
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function message(): string" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#reference
```

```diff main.tspp
@@ -1,4 +1,4 @@
 function message(): string {
          ^^^^^^^ definition
-    return "one";
+    return "two";
 }
@@ -9,3 +9,3 @@
 function unrelated(): int32 {
-    return 1;
+    return 2;
 }
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function message(): string" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#reference
```

```diff main.tspp
@@ -1,4 +1,4 @@
 function message(): string {
          ^^^^^^^ definition
-    return "two";
+    return "three";
 }
@@ -9,3 +9,3 @@
 function unrelated(): int32 {
-    return 2;
+    return 3;
 }
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function message(): string" location=main.tspp:1:1-3:2 selection=main.tspp#definition range=main.tspp#reference
```

## Documentation

### Include declaration documentation

Documentation comes from the referenced declaration.

```tspp main.tspp
/// Return the supplied name.
/// @typeParam Value - The supplied value type.
/// @param name - The value to return.
/// @example
/// ```tspp
/// identity<string>("Destack");
/// ```
function identity<Value>(name: Value): Value {
         ^^^^^^^^ definition
    return name;
}

const name = identity<string>("Destack");
             ^^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="function identity<Value>(name: Value): Value" type="(name: string) => string" documentation="Return the supplied name.\n\n## Type parameters\n\n- `Value`: The supplied value type.\n\n## Parameters\n\n- `name`: The value to return.\n\n## Examples\n\n```tspp\nidentity<string>(\"Destack\");\n```" location=main.tspp:8:1-10:2 selection=main.tspp#definition range=main.tspp#reference
```

### Return no symbol hover inside documentation

Documentation belongs to its declaration but is not itself a symbol occurrence.

```tspp main.tspp
/// Return the supplied name.
    ^^^^^^ documentation
function identity(name: string): string {
    return name;
}
```

```query hover main.tspp#documentation
@hover.none
```

### Include expression documentation

Documentation attached to an expression is available without a symbol declaration.

```tspp main.tspp
const result =
    /// Computed value.
    42;
    ^^ expression
```

```query hover main.tspp#expression
@hover.documentation text="Computed value." range=main.tspp#expression
```

## Types

### Hover over a type alias

A type reference reports its declaration kind and name.

```tspp main.tspp
type UserId = int32;

declare const user: UserId;
                    ^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="type UserId = int32" location=main.tspp:1:1-1:20 selection=main.tspp:1:6-1:12 range=main.tspp#reference
```

### Hover over a class

A class reference reports its declaration shape.

```tspp main.tspp
class Service {}

declare const service: Service;
                       ^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="class Service" location=main.tspp:1:1-1:17 selection=main.tspp:1:7-1:14 range=main.tspp#reference
```

### Hover over an interface

An interface reference reports its declaration shape.

```tspp main.tspp
interface Drawable {
    draw(): void;
}

declare const drawable: Drawable;
                        ^^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="interface Drawable" location=main.tspp:1:1-3:2 selection=main.tspp:1:11-1:19 range=main.tspp#reference
```

### Hover over a newtype

A newtype reference reports its nominal declaration.

```tspp main.tspp
newtype UserId = int64;

declare const user: UserId;
                    ^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="newtype UserId = int64" location=main.tspp:1:1-1:23 selection=main.tspp:1:9-1:15 range=main.tspp#reference
```

### Hover over an intrinsic newtype

An intrinsic value remains part of the authored declaration signature.

```tspp main.tspp
export newtype Address<T> = intrinsic;
               ^^^^^^^ definition

declare const address: Address<uint8>;
                       ^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export newtype Address<T> = intrinsic" type="Address<uint8>" location=main.tspp:1:1-1:38 selection=main.tspp#definition range=main.tspp#reference
```

### Hover over every remaining nominal declaration kind

Structs, enums, and nominal interfaces report distinct declaration signatures.

```tspp main.tspp
struct Packet {}
       ^^^^^^ packet

enum Status { Ready }
     ^^^^^^ status

newtype interface Display {}
                  ^^^^^^^ display
```

```query hover main.tspp#packet
@hover.item index=0 declaration="struct Packet" location=main.tspp:1:1-1:17 selection=main.tspp#packet range=main.tspp#packet
```

```query hover main.tspp#status
@hover.item index=0 declaration="enum Status" location=main.tspp:3:1-3:22 selection=main.tspp#status range=main.tspp#status
```

```query hover main.tspp#display
@hover.item index=0 declaration="newtype interface Display" location=main.tspp:5:1-5:29 selection=main.tspp#display range=main.tspp#display
```

### Include generic parameters in type declarations

A generic type reference reports the declaration's complete generic header.

```tspp main.tspp
class Box<Value> {}

declare const box: Box<int32>;
                   ^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="class Box<Value>" type="Box<int32>" location=main.tspp:1:1-1:20 selection=main.tspp:1:7-1:10 range=main.tspp#reference
```

### Hover over extension type parameters

An extension type parameter reports its declaration at both declaration and reference sites.

```tspp main.tspp
newtype Box<Value> = Value;

extension<Element> of Box<Element> {}
          ^^^^^^^ declaration
                          ^^^^^^^ reference
```

```query hover main.tspp#declaration
@hover.item index=0 declaration=Element location=main.tspp#declaration range=main.tspp#declaration
```

```query hover main.tspp#reference
@hover.item index=0 declaration=Element location=main.tspp#declaration range=main.tspp#reference
```

### Render the current type alias

Hover uses the declaration selected after each edit.

```tspp main.tspp
type Value = int32;
     ^^^^^ definition

declare const value: Value;
                     ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="type Value = int32" location=main.tspp:1:1-1:19 selection=main.tspp#definition range=main.tspp#reference
```

```tspp main.tspp change
type Value = string;
     ^^^^^ definition

declare const value: Value;
                     ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="type Value = string" location=main.tspp:1:1-1:20 selection=main.tspp#definition range=main.tspp#reference
```

```diff main.tspp
@@ -1,2 +1,2 @@
-type Value = string;
+type Value = boolean;
      ^^^^^ definition
```

```query hover main.tspp#reference
@hover.item index=0 declaration="type Value = boolean" location=main.tspp:1:1-1:21 selection=main.tspp#definition range=main.tspp#reference
```

### Render a renamed extension type parameter

Extension parameter hover updates at its declaration and reference.

```tspp main.tspp
newtype Box<Value> = Value;

extension<Element> of Box<Element> {}
          ^^^^^^^ declaration
                          ^^^^^^^ reference
```

```query hover main.tspp#declaration
@hover.item index=0 declaration=Element location=main.tspp#declaration range=main.tspp#declaration
```

```query hover main.tspp#reference
@hover.item index=0 declaration=Element location=main.tspp#declaration range=main.tspp#reference
```

```diff main.tspp
@@ -3,3 +3,3 @@
-extension<Element> of Box<Element> {}
-          ^^^^^^^ declaration
-                          ^^^^^^^ reference
+extension<Item> of Box<Item> {}
+          ^^^^ declaration
+                       ^^^^ reference
```

```query hover main.tspp#declaration
@hover.item index=0 declaration=Item location=main.tspp#declaration range=main.tspp#declaration
```

```query hover main.tspp#reference
@hover.item index=0 declaration=Item location=main.tspp#declaration range=main.tspp#reference
```

## Members

### Hover over a field access

A field access reports its owning type and field type.

```tspp main.tspp
struct Point {
    x: int32;
    ^ definition
}

function read(point: Point): int32 {
    return point.x;
                 ^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="Point.x: int32" location=main.tspp:2:5-2:13 selection=main.tspp#definition range=main.tspp#reference
```

```query hover main.tspp#definition
@hover.item index=0 declaration="Point.x: int32" location=main.tspp:2:5-2:13 selection=main.tspp#definition range=main.tspp#definition
```

### Hover over a method

A method reports its container-qualified signature.

```tspp main.tspp
class Service {
    run(value: int32): void {}
    ^^^ definition
}

function start(service: Service): void {
    service.run(1);
            ^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="Service.run(value: int32): void" location=main.tspp:2:5-2:31 selection=main.tspp#definition range=main.tspp#reference
```

```query hover main.tspp#definition
@hover.item index=0 declaration="Service.run(value: int32): void" location=main.tspp:2:5-2:31 selection=main.tspp#definition range=main.tspp#definition
```

### Hover over an extension method

An extension call reports its method and target type.

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

```query hover main.tspp#reference
@hover.item index=0 declaration="Calculator.add(left: int32, right: int32): int32" location=main.tspp:4:5-6:6 selection=main.tspp:4:5-4:8 range=main.tspp#reference
```

### Hover over an associated constant

An associated constant access reports its container and type.

```tspp main.tspp
struct Buffer {
    const Width: uint = 8;
}

const width = Buffer.Width;
                     ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="Buffer.Width: uint64" location=main.tspp:2:5-2:26 selection=main.tspp:2:11-2:16 range=main.tspp#reference
```

### Hover over associated types

Associated type hovers preserve required, constrained, and defaulted declarations.

```tspp main.tspp
interface Types {
    type Required;
         ^^^^^^^^ required
    type Constrained: string;
         ^^^^^^^^^^^ constrained
    type Defaulted: string = string;
         ^^^^^^^^^ defaulted
}
```

```query hover main.tspp#required
@hover.item index=0 declaration=Types.Required location=main.tspp:2:5-2:18 selection=main.tspp#required range=main.tspp#required
```

```query hover main.tspp#constrained
@hover.item index=0 declaration="Types.Constrained: string" location=main.tspp:3:5-3:29 selection=main.tspp#constrained range=main.tspp#constrained
```

```query hover main.tspp#defaulted
@hover.item index=0 declaration="Types.Defaulted: string = string" location=main.tspp:4:5-4:36 selection=main.tspp#defaulted range=main.tspp#defaulted
```

### Include method documentation

Method hover includes documentation from the member declaration.

```tspp main.tspp
class Service {
    /// Start one task.
    run(value: int32): void {}
}

function start(service: Service): void {
    service.run(1);
            ^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="Service.run(value: int32): void" documentation="Start one task." location=main.tspp:3:5-3:31 selection=main.tspp:3:5-3:8 range=main.tspp#reference
```

### Hover over an enum member

An enum member reports its container-qualified value.

```tspp main.tspp
enum Color {
    Red,
}

const color = Color.Red;
                    ^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="Color.Red: Color.Red" location=main.tspp:2:5-2:8 range=main.tspp#reference
```

## Parameters

### Hover over a function parameter

A parameter reference reports its parameter type.

```tspp main.tspp
function identity(value: string): string {
    return value;
           ^^^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="value: string" location=main.tspp:1:19-1:32 selection=main.tspp:1:19-1:24 range=main.tspp#reference
```

## Locals

### Hover over an inferred binding

A local binding reports its inferred type.

```tspp main.tspp
function read(): int32 {
    const count = 1;
          ^^^^^ definition
    return count;
           ^^^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="const count: int32" location=main.tspp#definition range=main.tspp#reference
```

### Preserve borrowed handle storage in inferred types

```tspp main.tspp
class User {}
function read<'a>(value: &'a readonly User): void {
    const copy = value;
          ^^^^ definition
    copy;
    ^^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="const copy: &'a readonly User" location=main.tspp#definition range=main.tspp#reference
```

### Print an open callable receiver mode

A function value whose receiver mode is a parameter prints that parameter.

```tspp main.tspp
function call<const M: ReceiverMode>(callback: Function<(), void, M>): void {
    const copy = callback;
          ^^^^ definition
    copy;
    ^^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="const copy: Function<(), void, M>" location=main.tspp#definition range=main.tspp#reference
```

### Preserve immutable borrows in inferred types

```tspp main.tspp
function inspect<'a>(value: &'a immutable int32): void {
    const copy = value;
          ^^^^ definition
    copy;
    ^^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="const copy: &'a immutable int32" location=main.tspp#definition range=main.tspp#reference
```

### Preserve exclusive borrows in inferred types

```tspp main.tspp
function update<'a>(value: &'a exclusive int32): void {
    const copy = value;
          ^^^^ definition
    copy;
    ^^^^ reference
}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="const copy: &'a exclusive int32" location=main.tspp#definition range=main.tspp#reference
```

## Imports

### Hover over an imported function

Imported references use the declaration signature from the defining module.

```tspp library.tspp
export function greet(): string {
    return "one";
}
```

```tspp main.tspp
import { greet } from "./library.tspp";

greet();
^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export function greet(): string" location=library.tspp:1:1-3:2 selection=library.tspp:1:17-1:22 range=main.tspp#reference
```

```diff library.tspp
@@ -1,3 +1,3 @@
 export function greet(): string {
-    return "one";
+    return "two";
 }
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export function greet(): string" location=library.tspp:1:1-3:2 selection=library.tspp:1:17-1:22 range=main.tspp#reference
```

```diff library.tspp
@@ -1,3 +1,3 @@
-export function greet(): string {
+export function welcome(): string {
     return "two";
 }
```

```query hover main.tspp#reference
@hover.none
```

### Hover over a builtin package declaration

Builtin imports retain their declaration and documentation.

```tspp main.tspp
import { log } from "tspp:console";

log("ready");
^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export function log(...values: unknown[]): Result<void, HostError>" documentation="Write a line to stdout." location=tspp://console/console:105:1-107:2 selection=tspp://console/console:105:17-105:20 range=main.tspp#reference
```

### Hover through a re-export

A re-export resolves to the original declaration.

```tspp library.tspp
export default function scale(value: int32, factor: int32): int32 {
    return value * factor;
}
```

```tspp barrel.tspp
export { default as scale } from "./library.tspp";
```

```tspp main.tspp
import { scale } from "./barrel.tspp";

const result = scale(2, 3);
               ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export default function scale(value: int32, factor: int32): int32" location=library.tspp:1:1-3:2 selection=library.tspp:1:25-1:30 range=main.tspp#reference
```

### Include documentation through a re-export

A re-exported function reports its declaration signature and documentation.

```tspp library.tspp
/// Scale one value.
export default function scale(value: int32, factor: int32): int32 {
    return value * factor;
}
```

```tspp barrel.tspp
export { default as scale } from "./library.tspp";
```

```tspp main.tspp
import { scale } from "./barrel.tspp";

const result = scale(2, 3);
               ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export default function scale(value: int32, factor: int32): int32" documentation="Scale one value." location=library.tspp:2:1-4:2 selection=library.tspp:2:25-2:30 range=main.tspp#reference
```

### Hover over a default import

A default import reports the declaration signature from its defining module.

```tspp library.tspp
export default function greet(name: string): string {
    return name;
}
```

```tspp main.tspp
import welcome from "./library.tspp";

const message = welcome("Destack");
                ^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export default function greet(name: string): string" location=library.tspp:1:1-3:2 selection=library.tspp:1:25-1:30 range=main.tspp#reference
```

### Hover over a namespace member

A namespace member reports the declaration signature from its defining module.

```tspp library.tspp
export function greet(name: string): string {
    return name;
}
```

```tspp main.tspp
import * as library from "./library.tspp";

const message = library.greet("Destack");
                        ^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="export function greet(name: string): string" location=library.tspp:1:1-3:2 selection=library.tspp:1:17-1:22 range=main.tspp#reference
```

## Globals

### Hover over global type and value references

Global exports retain their defining declarations across modules.

```json package.json
{
  "packageManager": "tspp@2026.9.0",
  "name": "@test/query",
  "compiler": {
    "globals": ["global.tspp"]
  },
  "targets": {
    "default": {
      "include": ["**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
```

```tspp library.tspp
export type BuiltinType = string;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ type_declaration
            ^^^^^^^^^^^ type_definition
export const builtinValue: int32 = 1;
             ^^^^^^^^^^^^ value_definition
```

```tspp global.tspp
global {
    export { BuiltinType, builtinValue } from "./library.tspp";
}
```

```tspp main.tspp
declare const typed: BuiltinType;
                     ^^^^^^^^^^^ type_reference

const used = builtinValue;
             ^^^^^^^^^^^^ value_reference
```

```query hover main.tspp#type_reference
@hover.item index=0 declaration="export type BuiltinType = string" location=library.tspp:1:1-1:33 selection=library.tspp#type_definition range=main.tspp#type_reference
```

```query hover main.tspp#value_reference
@hover.item index=0 declaration="const builtinValue: int32" location=library.tspp#value_definition range=main.tspp#value_reference
```

## Decorators

### Include decorators attached to a declaration

A hovered symbol shows its declaration decorators in source order.

```tspp main.tspp
newtype marker = (string,);

@marker("service")
interface Service {}
          ^^^^^^^ definition

declare const service: Service;
                       ^^^^^^^ reference
```

```query hover main.tspp#reference
@hover.item index=0 declaration="@marker(\"service\")\ninterface Service" location=main.tspp:4:1-4:21 selection=main.tspp#definition range=main.tspp#reference
```

### Hover over a decorator reference

A decorator target reports its newtype declaration.

```tspp main.tspp
newtype marker = (string,);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ definition

@marker("value")
 ^^^^^^ reference
function run(): void {}
```

```query hover main.tspp#reference
@hover.item index=0 declaration="newtype marker = (string,)" location=main.tspp:1:1-1:27 selection=main.tspp#definition range=main.tspp#reference
```

### Hover over a global decorator

A global decorator resolves to its standard-library declaration.

```tspp main.tspp
@languageItem()
 ^^^^^^^^^^^^ reference
newtype Marker = string;
```

```query hover main.tspp#reference
@hover.item index=0 declaration="@languageItem(\"decorator.languageItem\")\nexport newtype languageItem = (string,) | ()" documentation="Compiler language item marker." location=tspp://decorator/intrinsic:7:1-7:45 selection=tspp://decorator/intrinsic:7:16-7:28 range=main.tspp#reference
```

## Empty Results

### Return no hover for a lambda

A lambda has no name to hover.

```tspp main.tspp
const transform = (value: string): string => value;
                  ^ lambda
```

```query hover main.tspp#lambda
@hover.none
```

### Return no hover for a literal

An ordinary literal has no symbol hover.

```tspp main.tspp
const value = 42;
              ^^ literal
```

```query hover main.tspp#literal
@hover.none
```
