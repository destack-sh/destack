# Inlay Hints

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

Hints retain source order across multiple declarators.

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

An inferred generic value retains its selected type arguments.

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

An inferred branch value retains every checked union member.

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

## Call Arguments

### Show parameter names for local call arguments

Call arguments receive names from the selected callable.

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
@inlay_hints.hint position=main.ds#left_argument@start label="left:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#right_argument@start label="right:" kind=parameter padding_right=true
```

### Show parameter names for imported call arguments

Imported call arguments retain parameter names from the defining module.

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
@inlay_hints.hint position=main.ds#color_argument@start label="color:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#coats_argument@start label="coats:" kind=parameter padding_right=true
```

### Show parameter names for method arguments

Method arguments use names from the selected method.

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
@inlay_hints.hint position=main.ds#argument@start label="name:" kind=parameter padding_right=true
```

### Show parameter names for extension arguments

Extension calls use the parameter names from the selected extension member.

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
@inlay_hints.hint position=main.ds#offset_argument@start label="offset:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#length_argument@start label="length:" kind=parameter padding_right=true
```

### Use the selected overload parameter

Overload calls use only the exact callable selected by checking.

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
@inlay_hints.hint position=main.ds#argument@start label="text:" kind=parameter padding_right=true
```

### Use selected generic parameter names

Generic instantiation preserves the authored parameter name.

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
@inlay_hints.hint position=main.ds#result@end label=": string" kind=type
@inlay_hints.hint position=main.ds#argument@start label="value:" kind=parameter padding_right=true
```

### Show one rest parameter for every bound argument

Arguments bound into a rest parameter share its authored name.

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
@inlay_hints.hint position=main.ds#first_argument@start label="first:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#first_rest_argument@start label="values:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#second_rest_argument@start label="values:" kind=parameter padding_right=true
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
@inlay_hints.hint position=main.ds#second_argument@start label="greeting:" kind=parameter padding_right=true
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

### Omit parameter names from named arguments

Named arguments already carry their parameter names.

```ds main.ds
function greet(name: string, greeting: string): void {}

greet(name: "World", greeting: "Hello");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ call
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

When exact union call targets disagree on the parameter name, no name is invented.

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

Namespace member calls retain the selected exported parameter names.

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
@inlay_hints.hint position=main.ds#color_argument@start label="color:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#coats_argument@start label="coats:" kind=parameter padding_right=true
```

### Show parameter names through re-exports

Re-export aliases retain the declared callable parameter names.

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
@inlay_hints.hint position=main.ds#value_argument@start label="value:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#factor_argument@start label="factor:" kind=parameter padding_right=true
```

### Show parameter names through a default import

Default import aliases retain the declaration's authored parameters.

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
@inlay_hints.hint position=main.ds#text_argument@start label="text:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#count_argument@start label="count:" kind=parameter padding_right=true
```

## Construction

### Show class constructor parameter names

Class construction uses parameters from the selected constructor.

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
@inlay_hints.hint position=main.ds#name_argument@start label="name:" kind=parameter padding_right=true
@inlay_hints.hint position=main.ds#age_argument@start label="age:" kind=parameter padding_right=true
```

### Show a newtype backing value parameter

Newtype construction exposes its single checked value parameter.

```ds main.ds
newtype UserId = string;

const userId = UserId("user-1");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^^^ user_id
                      ^^^^^^^^ argument
```

```query inlay_hints main.ds#construction
@inlay_hints.hint position=main.ds#user_id@end label=": UserId" kind=type
@inlay_hints.hint position=main.ds#argument@start label="value:" kind=parameter padding_right=true
```

### Omit a tagged payload hint without a parameter name

A structural tagged payload does not acquire an invented parameter name.

```ds main.ds
@derive(Tagged)
newtype Status = Ok<string>;

const status = Status.Ok({ value: "ready" });
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ construction
      ^^^^^^ status
```

```query inlay_hints main.ds#construction
@inlay_hints.hint position=main.ds#status@end label=": Status" kind=type
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
