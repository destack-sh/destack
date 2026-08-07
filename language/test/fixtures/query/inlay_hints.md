
## Inferred Bindings

### Show an inferred binding type

An unannotated binding receives its inferred type as a hint.

```ds main.ds
const count = 1;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": 1" kind=type
```

### Omit an explicit binding type

An explicit type annotation needs no redundant hint.

```ds main.ds
const count: int32 = 1;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.none
```

### Show every inferred binding in the range

Hints follow source order across multiple declarators.

```ds main.ds
const first = 1, second = 2;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ line
      ^^^^^ first
                 ^^^^^^ second
```

```query inlay_hints main.ds#line
@inlay_hints.hint position=main.ds#first@end label=": 1" kind=type
@inlay_hints.hint position=main.ds#second@end label=": 2" kind=type
```

### Show an inferred nominal type

A constructed class binding receives its inferred nominal type.

```ds main.ds
class Dog {}

const pet = new Dog();
      ^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": Dog" kind=type
```

### Show an applied generic type

An inferred generic value displays its applied type arguments.

```ds main.ds
struct Box<Value> {
    value: Value;
}

const box = Box<string> { value: "ready" };
      ^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": Box<string>" kind=type
```

### Show an inferred union type

An inferred branch value displays every union member.

```ds main.ds
struct Circle {}
struct Square {}

declare const condition: boolean;
declare const circle: Circle;
declare const square: Square;

const shape = condition ? circle : square;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": Circle | Square" kind=type
```

### Show an inferred callable type

Function-valued bindings display their complete callable type.

```ds main.ds
const predicate = (value: int32): boolean => value > 0;
      ^^^^^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": (value: int32) => boolean" kind=type
```

### Omit destructured binding types

Destructuring does not receive one misleading aggregate type hint.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

const { x, y } = Point { x: 1, y: 2 };
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.none
```

### Restrict hints to the requested range

A range query does not return hints for neighboring declarations.

```ds main.ds
const first = 1;
      ^^^^^ first
const second = "two";
      ^^^^^^ second
```

```query inlay_hints main.ds#second
@inlay_hints.hint position=main.ds#second@end label=": \"two\"" kind=type
```

### Request only inferred type hints

Disabling parameter hints leaves inferred type hints unchanged.

```ds main.ds
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^ result
```

```query inlay_hints main.ds#call parameter_hints=false
@inlay_hints.hint position=main.ds#result@end label=": int32" kind=type
```

### Request only parameter name hints

Disabling type hints leaves parameter name hints unchanged.

```ds main.ds
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
                        ^ argument
```

```query inlay_hints main.ds#call type_hints=false
@inlay_hints.hint position=main.ds#argument label="value:" kind=parameter padding_right=true
```

### Disable every hint family

Disabling every hint family returns no hints.

```ds main.ds
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
```

```query inlay_hints main.ds#call type_hints=false parameter_hints=false
@inlay_hints.none
```

## Call Arguments

### Show parameter names for local call arguments

Call arguments receive names from their callable.

```ds main.ds
function add(left: int32, right: int32): int32 {
    return left + right;
}

const result = add(1, 2);
^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^ result
                   ^ left_argument
                      ^ right_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#result@end label=": int32" kind=type
@inlay_hints.hint position=main.ds#left_argument label="left:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#right_argument label="right:" kind=parameter padding_right=true
```

### [ignored] Show parameter names for callable values

Calls through inferred callable bindings use the lambda parameter names.

```ds main.ds
const transform = (value: int32): int32 => value;

transform(1);
^^^^^^^^^^^^^ call
          ^ argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#argument label="value:" kind=parameter padding_right=true
```

### [ignored] Show parameter names for callable parameters

A call through a function parameter uses names from its function type.

```ds main.ds
function apply(callback: (value: string) => string): string {
    return callback("ready");
                    ^^^^^^^ argument
}
```

```query inlay_hints main.ds#argument
@inlay_hints.hint position=main.ds#argument label="value:" kind=parameter padding_right=true
```

### Show parameter names for imported call arguments

Imported call arguments use parameter names from the defining module.

```ds library.ds
export function paint(color: string, coats: int32): void {}
```

```ds main.ds
import { paint } from "./library.ds";

paint("blue", 2);
^^^^^^^^^^^^^^^^^ call
      ^^^^^^ color_argument
              ^ coats_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#color_argument label="color:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#coats_argument label="coats:" kind=parameter padding_right=true
```

### Show parameter names for method arguments

Method arguments use names from their method.

```ds main.ds
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

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#message@end label=": string" kind=type
@inlay_hints.hint position=main.ds#argument label="name:" kind=parameter padding_right=true
```

### Show parameter names for extension arguments

Extension calls use the parameter names from their extension method.

```ds main.ds
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

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#offset_argument label="offset:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#length_argument label="length:" kind=parameter padding_right=true
```

### Use the matching overload parameter

Overload calls use the parameters of the matching declaration.

```ds main.ds
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

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#argument label="text:" kind=parameter padding_right=true
```

### Use generic parameter names

Generic instantiation preserves the declared parameter name.

```ds main.ds
function identity<Value>(value: Value): Value {
    return value;
}

const result = identity("ready");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
      ^^^^^^ result
                        ^^^^^^^ argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#result@end label=": \"ready\"" kind=type
@inlay_hints.hint position=main.ds#argument label="value:" kind=parameter padding_right=true
```

### Show one rest parameter for every bound argument

Arguments bound into a rest parameter share its declared name.

```ds main.ds
function sum(first: int32, ...values: int32[]): int32 {
    return first;
}

sum(1, 2, 3);
^^^^^^^^^^^^^ call
    ^ first_argument
       ^ first_rest_argument
          ^ second_rest_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#first_argument label="first:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#first_rest_argument label="values:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#second_rest_argument label="values:" kind=parameter padding_right=true
```

### Omit parameter names repeated by arguments

An argument with the same identifier as its parameter is already clear.

```ds main.ds
function greet(name: string, greeting: string): void {}

const name: string = "World";
greet(name, "Hello");
^^^^^^^^^^^^^^^^^^^^ call
            ^^^^^^^ second_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#second_argument label="greeting:" kind=parameter padding_right=true
```

### Omit a repeated member name

The final member name can already identify its corresponding parameter.

```ds main.ds
struct Configuration {
    timeout: uint;
}

function connect(timeout: uint): void {}

declare const configuration: Configuration;
connect(configuration.timeout);
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
```

```query inlay_hints main.ds#call
@inlay_hints.none
```

### Omit parameter names from spread arguments

A spread expression does not correspond to one displayed parameter.

```ds main.ds
function add(left: int32, right: int32): int32 {
    return left + right;
}

const arguments: int32[] = [1, 2];
add(...arguments);
^^^^^^^^^^^^^^^^^^ call
```

```query inlay_hints main.ds#call
@inlay_hints.none
```

### Omit an ambiguous union parameter name

When union call targets disagree on a parameter name, no name is invented.

```ds main.ds
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

```query inlay_hints main.ds#call
@inlay_hints.none
```

### Show parameter names through namespace imports

Namespace member calls use the exported parameter names.

```ds library.ds
export function paint(color: string, coats: int32): void {}
```

```ds main.ds
import * as library from "./library.ds";

library.paint("blue", 2);
^^^^^^^^^^^^^^^^^^^^^^^^^ call
              ^^^^^^ color_argument
                      ^ coats_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#color_argument label="color:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#coats_argument label="coats:" kind=parameter padding_right=true
```

### Show parameter names through re-exports

Re-export aliases use the declared callable parameter names.

```ds library.ds
export default function scale(value: int32, factor: int32): int32 {
    return value * factor;
}
```

```ds barrel.ds
export { default as scale } from "./library.ds";
```

```ds main.ds
import { scale } from "./barrel.ds";

scale(2, 3);
^^^^^^^^^^^^ call
      ^ value_argument
         ^ factor_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#value_argument label="value:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#factor_argument label="factor:" kind=parameter padding_right=true
```

### Show parameter names through a default import

Default import aliases use the declaration's parameter names.

```ds library.ds
export default function repeat(text: string, count: uint): string {
    return text;
}
```

```ds main.ds
import repeat from "./library.ds";

repeat("ready", 2);
^^^^^^^^^^^^^^^^^^^ call
       ^^^^^^^ text_argument
                ^ count_argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#text_argument label="text:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#count_argument label="count:" kind=parameter padding_right=true
```

## Construction

### Show class constructor parameter names

Class construction uses parameters from its constructor.

```ds main.ds
class User {
    constructor(name: string, age: uint) {}
}

const user = new User("Ada", 42);
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^ user
                      ^^^^^ name_argument
                             ^^ age_argument
```

```query inlay_hints main.ds#construction
@inlay_hints.hint position=main.ds#user@end label=": User" kind=type
@inlay_hints.hint position=main.ds#name_argument label="name:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#age_argument label="age:" kind=parameter padding_right=true
```

### Omit parameter hints for newtype construction

A newtype constructor has no declared parameter name to display.

```ds main.ds
newtype UserId = string;

const userId = UserId("user-1");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^^^ user_id
```

```query inlay_hints main.ds#construction
@inlay_hints.hint position=main.ds#user_id@end label=": UserId" kind=type
```

### Omit a tagged payload hint without a parameter name

A tagged payload has no parameter name to display.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;

const status = Status.Ok({ value: "ready" });
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^^^ status
```

```query inlay_hints main.ds#construction
@inlay_hints.hint position=main.ds#status@end label=": Status.Ok" kind=type
```

### Return no parameter names for an empty call

A zero-argument call has no parameter positions to annotate.

```ds main.ds
function ping(): void {}

ping();
^^^^^^ call
```

```query inlay_hints main.ds#call
@inlay_hints.none
```

## Source changes

### Update an inferred binding hint

An inferred type hint reflects the current initializer.

```ds main.ds
const value = 1;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": 1" kind=type
```

```ds main.ds change
const value = true;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": true" kind=type
```

### Render inferred types during incomplete edits

An unresolved initializer renders as `<error>`.

```ds main.ds
const value = 1;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": 1" kind=type
```

```ds main.ds change
const value = missing;
      ^^^^^ binding
```

```query inlay_hints main.ds#binding
@inlay_hints.hint position=main.ds#binding@end label=": <error>" kind=type
```

### Update parameter names after a callable edit

Parameter name hints update when the callable parameter is renamed.

```ds main.ds
function send(value: string): void {}

send("ready");
^^^^^^^^^^^^^^ call
     ^ argument
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#argument label="value:" kind=parameter padding_right=true
```

```diff main.ds
@@ -1 +1 @@
-function send(value: string): void {}
+function send(message: string): void {}
```

```query inlay_hints main.ds#call
@inlay_hints.hint position=main.ds#argument label="message:" kind=parameter padding_right=true
```
