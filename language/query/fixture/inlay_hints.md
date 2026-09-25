
## Inferred Bindings

### Show an inferred binding type

An unannotated binding receives its inferred type as a hint.

```tspp main.tspp
const count = 1;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": 1" kind=type
```

### Omit an explicit binding type

An explicit type annotation needs no redundant hint.

```tspp main.tspp
const count: int32 = 1;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.none
```

### Show every inferred binding in the range

Hints follow source order across multiple declarators.

```tspp main.tspp
const first = 1, second = 2;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ line
      ^^^^^ first
                 ^^^^^^ second
```

```query inlay_hints main.tspp#line
@inlay_hints.hint position=main.tspp#first@end label=": 1" kind=type
@inlay_hints.hint position=main.tspp#second@end label=": 2" kind=type
```

### Show an inferred nominal type

A constructed class binding receives its inferred nominal type.

```tspp main.tspp
class Dog {}

const pet = new Dog();
      ^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": Dog" kind=type
```

### Show an applied generic type

An inferred generic value displays its applied type arguments.

```tspp main.tspp
struct Box<Value> {
    value: Value;
}

const box = Box<string> { value: "ready" };
      ^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": Box<string>" kind=type
```

### Show an inferred union type

An inferred branch value displays every union member.

```tspp main.tspp
struct Circle {}
struct Square {}

declare const condition: boolean;
declare const circle: Circle;
declare const square: Square;

const shape = condition ? circle : square;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": Circle | Square" kind=type
```

### Show an inferred callable type

Function-valued bindings display their complete callable type.

```tspp main.tspp
const predicate = (value: int32): boolean => value > 0;
      ^^^^^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": (value: int32) => boolean" kind=type
```

### Omit destructured binding types

Destructuring does not receive one misleading aggregate type hint.

```tspp main.tspp
struct Point {
    x: int32;
    y: int32;
}

const { x, y } = Point { x: 1, y: 2 };
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.none
```

### Restrict hints to the requested range

A requested range excludes hints for neighboring declarations.

```tspp main.tspp
const first = 1;
      ^^^^^ first
const second = "two";
      ^^^^^^ second
```

```query inlay_hints main.tspp#second
@inlay_hints.hint position=main.tspp#second@end label=": \"two\"" kind=type
```

### Request only inferred type hints

Disabling parameter hints leaves inferred type hints unchanged.

```tspp main.tspp
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^ result
```

```query inlay_hints main.tspp#call parameter_hints=false
@inlay_hints.hint position=main.tspp#result@end label=": int32" kind=type
```

### Request only parameter name hints

Disabling type hints leaves parameter name hints unchanged.

```tspp main.tspp
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
                        ^ argument
```

```query inlay_hints main.tspp#call type_hints=false
@inlay_hints.hint position=main.tspp#argument label="value:" kind=parameter padding_right=true
```

### Disable every hint family

Disabling every hint family returns no hints.

```tspp main.tspp
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
```

```query inlay_hints main.tspp#call type_hints=false parameter_hints=false
@inlay_hints.none
```

### Omit a hint for an explicit declaration while typing

Show inlay hints after every inserted character.

```tspp main.tspp
// module
```

```tspp main.tspp type
// module

declare const x: Clone;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
```

```query inlay_hints main.tspp#declaration
@inlay_hints.none
```

### Render the current inferred binding type

An inferred type hint reflects the current initializer.

```tspp main.tspp
const value = 1;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": 1" kind=type
```

```tspp main.tspp change
const value = true;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": true" kind=type
```

### Omit a statically absent binding hint

A false static gate receives no inferred type hint.

```tspp main.tspp
struct Position {
    x: float64;
    y: float64;
}

const position = Position { x: 1.0, y: 2.0 };
      ^^^^^^^^ position
const coordinate = position.y + position.x;
      ^^^^^^^^^^ coordinate
```

```query inlay_hints main.tspp#coordinate
@inlay_hints.hint position=main.tspp#coordinate@end label=": float64" kind=type
```

```diff main.tspp
@@ -8,2 +8,6 @@
 const coordinate = position.y + position.x;
       ^^^^^^^^^^ coordinate
+
+@if(false)
+const position = 5;
+      ^^^^^^^^ absent
```

```query inlay_hints main.tspp#absent
@inlay_hints.none
```

### Render an unresolved initializer type

An unresolved initializer renders as `<error>`.

```tspp main.tspp
const value = 1;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": 1" kind=type
```

```tspp main.tspp change
const value = missing;
      ^^^^^ binding
```

```query inlay_hints main.tspp#binding
@inlay_hints.hint position=main.tspp#binding@end label=": <error>" kind=type
```

### Render an inferred template literal type

An inferred hint preserves every static string segment and interpolated type.

```tspp main.tspp
declare const route: `api:${string}`;
const selected = route;
      ^^^^^^^^ selected
```

```query inlay_hints main.tspp#selected
@inlay_hints.hint position=main.tspp#selected@end label=": `api:${string}`" kind=type
```

## Call Arguments

### Show parameter names for local call arguments

Call arguments receive names from their callable.

```tspp main.tspp
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, 2);
^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^ result
                   ^ left_argument
                      ^ right_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#result@end label=": int32" kind=type
@inlay_hints.hint position=main.tspp#left_argument label="left:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#right_argument label="right:" kind=parameter padding_right=true
```

### Show parameter names for callable values

Calls through inferred callable bindings use the lambda parameter names.

```tspp main.tspp
const transform = (value: int32): int32 => value;

transform(1);
^^^^^^^^^^^^^ call
          ^ argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#argument label="value:" kind=parameter padding_right=true
```

### Show parameter names for callable parameters

A call through a function parameter uses names from its function type.

```tspp main.tspp
function apply(callback: (value: string) => string): string {
    return callback("ready");
                    ^^^^^^^ argument
}
```

```query inlay_hints main.tspp#argument
@inlay_hints.hint position=main.tspp#argument label="value:" kind=parameter padding_right=true
```

### Show parameter names for imported call arguments

Imported call arguments use parameter names from the defining module.

```tspp library.tspp
export function paint(color: string, coats: int32): void {}
```

```tspp main.tspp
import { paint } from "./library.tspp";

paint("blue", 2);
^^^^^^^^^^^^^^^^^ call
      ^^^^^^ color_argument
              ^ coats_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#color_argument label="color:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#coats_argument label="coats:" kind=parameter padding_right=true
```

### Show parameter names for method arguments

Method arguments use names from their method.

```tspp main.tspp
class Greeter {
    greet(name: string): string {
        return name;
    }
}

const greeter: Greeter = new Greeter();
const message = greeter.greet("World");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^^                         message
                              ^^^^^^^ argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#message@end label=": string" kind=type
@inlay_hints.hint position=main.tspp#argument label="name:" kind=parameter padding_right=true
```

### Show parameter names for extension arguments

Extension calls use the parameter names from their extension method.

```tspp main.tspp
struct Buffer {}

extension of Buffer {
    read(offset: uint, length: uint): void {}
}

declare const buffer: Buffer;
buffer.read(4, 8);
^^^^^^^^^^^^^^^^^^ call
            ^ offset_argument
               ^ length_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#offset_argument label="offset:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#length_argument label="length:" kind=parameter padding_right=true
```

### Use the matching overload parameter

Overload calls use the parameters of the matching declaration.

```tspp main.tspp
function parse(number: int32): int32 {
    return number;
}

function parse(text: string): string {
    return text;
}

parse("one");
^^^^^^^^^^^^^ call
      ^^^^^ argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#argument label="text:" kind=parameter padding_right=true
```

### Use generic parameter names

Generic instantiation preserves the declared parameter name.

```tspp main.tspp
function identity<Value>(value: Value): Value {
    return value;
}

const result = identity("ready");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^ result
                        ^^^^^^^ argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#result@end label=": \"ready\"" kind=type
@inlay_hints.hint position=main.tspp#argument label="value:" kind=parameter padding_right=true
```

### Show one rest parameter for every bound argument

Arguments bound into a rest parameter share its declared name.

```tspp main.tspp
function sum(first: int32, ...values: int32[]): int32 {
    return first;
}

sum(1, 2, 3);
^^^^^^^^^^^^^ call
    ^ first_argument
       ^ first_rest_argument
          ^ second_rest_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#first_argument label="first:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#first_rest_argument label="values:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#second_rest_argument label="values:" kind=parameter padding_right=true
```

### Omit parameter names repeated by arguments

An argument with the same identifier as its parameter is already clear.

```tspp main.tspp
function greet(name: string, greeting: string): void {}

const name: string = "World";
greet(name, "Hello");
^^^^^^^^^^^^^^^^^^^^ call
            ^^^^^^^ second_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#second_argument label="greeting:" kind=parameter padding_right=true
```

### Omit a repeated member name

The final member name can already identify its corresponding parameter.

```tspp main.tspp
struct Configuration {
    timeout: uint;
}

function connect(timeout: uint): void {}

declare const configuration: Configuration;
connect(configuration.timeout);
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
```

```query inlay_hints main.tspp#call
@inlay_hints.none
```

### Omit parameter names from spread arguments

A spread expression does not correspond to one displayed parameter.

```tspp main.tspp
function add(left: int32, right: int32): int32 {
    return left + right;
}

const arguments: int32[] = [1, 2];
add(...arguments);
^^^^^^^^^^^^^^^^^^ call
```

```query inlay_hints main.tspp#call
@inlay_hints.none
```

### Omit an ambiguous union parameter name

When union call targets disagree on a parameter name, no name is invented.

```tspp main.tspp
struct TextSink {
    write(text: string): void {}
}

struct MessageSink {
    write(message: string): void {}
}

function write(sink: TextSink | MessageSink): void {
    sink.write("ready");
    ^^^^^^^^^^^^^^^^^^^^ call
}
```

```query inlay_hints main.tspp#call
@inlay_hints.none
```

### Show parameter names through namespace imports

Namespace member calls use the exported parameter names.

```tspp library.tspp
export function paint(color: string, coats: int32): void {}
```

```tspp main.tspp
import * as library from "./library.tspp";

library.paint("blue", 2);
^^^^^^^^^^^^^^^^^^^^^^^^^ call
              ^^^^^^ color_argument
                      ^ coats_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#color_argument label="color:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#coats_argument label="coats:" kind=parameter padding_right=true
```

### Show parameter names through re-exports

Re-export aliases use the declared callable parameter names.

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

scale(2, 3);
^^^^^^^^^^^^ call
      ^ value_argument
         ^ factor_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#value_argument label="value:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#factor_argument label="factor:" kind=parameter padding_right=true
```

### Show parameter names through a default import

Default import aliases use the declaration's parameter names.

```tspp library.tspp
export default function repeat(text: string, count: uint): string {
    return text;
}
```

```tspp main.tspp
import repeat from "./library.tspp";

repeat("ready", 2);
^^^^^^^^^^^^^^^^^^^ call
       ^^^^^^^ text_argument
                ^ count_argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#text_argument label="text:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#count_argument label="count:" kind=parameter padding_right=true
```

### Render the current parameter name

Parameter name hints update when the callable parameter is renamed.

```tspp main.tspp
function send(value: string): void {}

send("ready");
^^^^^^^^^^^^^^ call
     ^ argument
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#argument label="value:" kind=parameter padding_right=true
```

```diff main.tspp
@@ -1 +1 @@
-function send(value: string): void {}
+function send(message: string): void {}
```

```query inlay_hints main.tspp#call
@inlay_hints.hint position=main.tspp#argument label="message:" kind=parameter padding_right=true
```

## Construction

### Show class constructor parameter names

Class construction uses parameters from its constructor.

```tspp main.tspp
class User {
    constructor(name: string, age: uint) {}
}

const user = new User("Ada", 42);
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^ user
                      ^^^^^ name_argument
                             ^^ age_argument
```

```query inlay_hints main.tspp#construction
@inlay_hints.hint position=main.tspp#user@end label=": User" kind=type
@inlay_hints.hint position=main.tspp#name_argument label="name:" kind=parameter padding_right=true
@inlay_hints.hint position=main.tspp#age_argument label="age:" kind=parameter padding_right=true
```

### Omit parameter hints for newtype construction

A newtype constructor has no declared parameter name to display.

```tspp main.tspp
newtype UserId = string;

const userId = UserId("user-1");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^^^ user_id
```

```query inlay_hints main.tspp#construction
@inlay_hints.hint position=main.tspp#user_id@end label=": UserId" kind=type
```

### Return no parameter names for an empty call

A zero-argument call has no parameter positions to annotate.

```tspp main.tspp
function ping(): void {}

ping();
^^^^^^ call
```

```query inlay_hints main.tspp#call
@inlay_hints.none
```
