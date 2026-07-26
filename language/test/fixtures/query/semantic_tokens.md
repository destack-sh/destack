# Semantic Tokens

## Symbols

### Classify declarations and references

Semantic tokens add resolved symbol roles to the grammar's lexical highlighting.

```ds main.ds
function identity(value: int32): int32 {
         ^^^^^^^^ function
                  ^^^^^ parameter
    return value;
           ^^^^^ reference
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#function type=function modifiers=declaration
@semantic_tokens.token range=main.ds#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.ds#reference type=parameter
```

### Classify nominal declarations and members

Nominal declarations and their members retain distinct token kinds.

```ds main.ds
struct Point {
       ^^^^^ point
    x: int32;
    ^ field
}

class Shape {}
      ^^^^^ shape

enum Color {
     ^^^^^ color
    Red,
    ^^^ red
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#point type=struct modifiers=declaration
@semantic_tokens.token range=main.ds#field type=property modifiers=declaration
@semantic_tokens.token range=main.ds#shape type=class modifiers=declaration
@semantic_tokens.token range=main.ds#color type=enum modifiers=declaration
@semantic_tokens.token range=main.ds#red type=enum_member modifiers=declaration,readonly
```

### [ignored] Classify implicit interface member abstraction

An interface method is abstract even when it omits an explicit modifier.

```ds main.ds
interface Drawable {
          ^^^^^^^^ drawable
    draw(): void;
    ^^^^ method
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#drawable type=interface modifiers=declaration
@semantic_tokens.token range=main.ds#method type=method modifiers=declaration,abstract
```

### Classify every nominal declaration kind

Type aliases, newtypes, nominal interfaces, and extensions retain distinct roles.

```ds main.ds
type Identifier = uint64;
     ^^^^^^^^^^ type_alias

newtype UserId = Identifier;
        ^^^^^^ newtype
                 ^^^^^^^^^^ newtype_value

newtype interface Display {}
                  ^^^^^^^ nominal_interface

extension BufferAccess of UserId {}
          ^^^^^^^^^^^^ extension
                          ^^^^^^ extension_target
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#type_alias type=type modifiers=declaration
@semantic_tokens.token range=main.ds#newtype type=type modifiers=declaration
@semantic_tokens.token range=main.ds#newtype_value type=type
@semantic_tokens.token range=main.ds#nominal_interface type=interface modifiers=declaration
@semantic_tokens.token range=main.ds#extension type=type modifiers=declaration
@semantic_tokens.token range=main.ds#extension_target type=type
```

### Classify generic declarations and references

Generic type and comptime value parameters remain distinct from nominal types and locals.

```ds main.ds
function identity<Value, comptime size: usize>(value: Value): Value {
         ^^^^^^^^ function
                  ^^^^^ generic
                                  ^^^^ size_declaration
                                               ^^^^^ parameter
                                                      ^^^^^ parameter_type
                                                              ^^^^^ return_type
    const buffer: [uint8; size] = [];
          ^^^^^^ buffer
                          ^^^^ size_reference
    return value;
           ^^^^^ value_reference
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#function type=function modifiers=declaration
@semantic_tokens.token range=main.ds#generic type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.ds#size_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.ds#parameter_type type=type_parameter
@semantic_tokens.token range=main.ds#return_type type=type_parameter
@semantic_tokens.token range=main.ds#buffer type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#size_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.ds#value_reference type=parameter
```

## Imports

### Classify imported names and aliases

An imported name and its local alias use the canonical exported symbol kind.

```ds library.ds
export function paint(): void {}
```

```ds main.ds
import { paint as render } from "./library.ds";
         ^^^^^ imported
                  ^^^^^^ declaration

render();
^^^^^^ reference
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#imported type=function
@semantic_tokens.token range=main.ds#declaration type=function modifiers=declaration
@semantic_tokens.token range=main.ds#reference type=function
```

### Classify type and namespace imports

Plain and namespace aliases retain their canonical symbol kinds.

```ds library.ds
export struct Packet {}
```

```ds main.ds
import { Packet as Message } from "./library.ds";
         ^^^^^^ imported_type
                   ^^^^^^^ type_alias
import * as encoding from "./library.ds";
            ^^^^^^^^ namespace_alias

declare const message: Message;
              ^^^^^^^ message_declaration
                       ^^^^^^^ type_reference
encoding;
^^^^^^^^ namespace_reference
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#imported_type type=struct
@semantic_tokens.token range=main.ds#type_alias type=struct modifiers=declaration
@semantic_tokens.token range=main.ds#namespace_alias type=namespace modifiers=declaration
@semantic_tokens.token range=main.ds#message_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#type_reference type=struct
@semantic_tokens.token range=main.ds#namespace_reference type=namespace
```

## Bindings

### Classify using bindings

Resource bindings are immutable declarations and references.

```ds main.ds
interface Dispose {}
          ^^^^^^^ dispose_interface

declare function open(): Dispose;
                 ^^^^ open_declaration
                         ^^^^^^^ dispose_reference

using resource = open();
      ^^^^^^^^ using_declaration
                 ^^^^ open_reference
resource;
^^^^^^^^ using_reference
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#dispose_interface type=interface modifiers=declaration
@semantic_tokens.token range=main.ds#open_declaration type=function modifiers=declaration
@semantic_tokens.token range=main.ds#dispose_reference type=interface
@semantic_tokens.token range=main.ds#using_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#open_reference type=function
@semantic_tokens.token range=main.ds#using_reference type=variable modifiers=readonly
```

### Classify match bindings

Match-arm bindings remain immutable inside their guard and body.

```ds main.ds
const pair = (1, 2);
      ^^^^ pair_declaration
const result = match (pair) {
      ^^^^^^ result_declaration
                      ^^^^ pair_reference
    (left, right) => left + right
     ^^^^ left_declaration
           ^^^^^ right_declaration
                     ^^^^ left_reference
                            ^^^^^ right_reference
};
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#pair_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#result_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#pair_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.ds#left_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#right_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#left_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.ds#right_reference type=variable modifiers=readonly
```

### Classify exact member references

Member references use checked member identities rather than generic property shapes.

```ds main.ds
class Buffer {
      ^^^^^^ class_declaration
    readonly length: uint = 0;
             ^^^^^^ field_declaration

    read(offset: uint): uint8 {
    ^^^^ method_declaration
         ^^^^^^ offset_declaration
        return 0;
    }
}

function inspect(buffer: Buffer): uint {
         ^^^^^^^ function_declaration
                 ^^^^^^ buffer_declaration
                         ^^^^^^ type_reference
    const length = buffer.length;
          ^^^^^^ local_declaration
                   ^^^^^^ first_buffer_reference
                          ^^^^^^ field_reference
    buffer.read(0);
    ^^^^^^ second_buffer_reference
           ^^^^ method_reference
    return length;
           ^^^^^^ local_reference
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#class_declaration type=class modifiers=declaration
@semantic_tokens.token range=main.ds#field_declaration type=property modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#method_declaration type=method modifiers=declaration
@semantic_tokens.token range=main.ds#offset_declaration type=parameter modifiers=declaration
@semantic_tokens.token range=main.ds#function_declaration type=function modifiers=declaration
@semantic_tokens.token range=main.ds#buffer_declaration type=parameter modifiers=declaration
@semantic_tokens.token range=main.ds#type_reference type=class
@semantic_tokens.token range=main.ds#local_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#first_buffer_reference type=parameter
@semantic_tokens.token range=main.ds#field_reference type=property modifiers=readonly
@semantic_tokens.token range=main.ds#second_buffer_reference type=parameter
@semantic_tokens.token range=main.ds#method_reference type=method
@semantic_tokens.token range=main.ds#local_reference type=variable modifiers=readonly
```

### Classify reads and writes

Write occurrences add `modification` without inventing a second mutability modifier.

```ds main.ds
struct Counter {
       ^^^^^^^ counter_declaration
    value: int32;
    ^^^^^ field_declaration
}

function increment(counter: Counter): void {
         ^^^^^^^^^ function_declaration
                   ^^^^^^^ counter_parameter
                            ^^^^^^^ counter_type
    let amount = 1;
        ^^^^^^ local_declaration
    amount = amount + 1;
    ^^^^^^ local_write
             ^^^^^^ local_read
    counter.value = amount;
    ^^^^^^^ counter_reference
            ^^^^^ field_write
                    ^^^^^^ amount_read
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#counter_declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.ds#field_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.ds#function_declaration type=function modifiers=declaration
@semantic_tokens.token range=main.ds#counter_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.ds#counter_type type=struct
@semantic_tokens.token range=main.ds#local_declaration type=variable modifiers=declaration
@semantic_tokens.token range=main.ds#local_write type=variable modifiers=modification
@semantic_tokens.token range=main.ds#local_read type=variable
@semantic_tokens.token range=main.ds#counter_reference type=parameter
@semantic_tokens.token range=main.ds#field_write type=property modifiers=modification
@semantic_tokens.token range=main.ds#amount_read type=variable
```

## Modifiers

### Classify symbol attributes

Symbol attributes remain consistent between declarations and references.

```ds main.ds
export async function load(): void {}
                      ^^^^ async_declaration

const operation = load;
      ^^^^^^^^^ operation
                  ^^^^ async_reference

abstract class Base {
               ^^^^ abstract_class
    abstract run(): void;
             ^^^ abstract_method
}

class State {
      ^^^^^ state
    static readonly count: int32 = 0;
                    ^^^^^ static_declaration
}

const count = State.count;
      ^^^^^ local_declaration
              ^^^^^ state_reference
                    ^^^^^ static_reference
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#async_declaration type=function modifiers=declaration,async
@semantic_tokens.token range=main.ds#operation type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#async_reference type=function modifiers=async
@semantic_tokens.token range=main.ds#abstract_class type=class modifiers=declaration,abstract
@semantic_tokens.token range=main.ds#abstract_method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.ds#state type=class modifiers=declaration
@semantic_tokens.token range=main.ds#static_declaration type=property modifiers=declaration,readonly,static
@semantic_tokens.token range=main.ds#local_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.ds#state_reference type=class
@semantic_tokens.token range=main.ds#static_reference type=property modifiers=readonly,static
```

### Classify deprecated and default-library symbols

Deprecated state follows the symbol while built-in language items use `default_library`.

```ds main.ds
@deprecated("use verify")
 ^^^^^^^^^^ decorator
function validate(): void {}
         ^^^^^^^^ declaration

validate();
^^^^^^^^ reference
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#decorator type=decorator modifiers=default_library
@semantic_tokens.token range=main.ds#declaration type=function modifiers=declaration,deprecated
@semantic_tokens.token range=main.ds#reference type=function modifiers=deprecated
```

## Labels and Decorators

### [ignored] Classify labels

Label declarations and references retain their dedicated token kind.

```ds main.ds
outer: loop {
^^^^^ declaration
    break outer;
          ^^^^^ reference
}
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#declaration type=label modifiers=declaration
@semantic_tokens.token range=main.ds#reference type=label
```

### Classify user-defined decorators

Decorator applications remain distinct from the symbols that define them.

```ds main.ds
newtype tracked = ();
        ^^^^^^^ annotation_definition

@tracked
 ^^^^^^^ decorator
function start(): void {}
         ^^^^^ function
```

```query semantic_tokens main.ds
@semantic_tokens.token range=main.ds#annotation_definition type=type modifiers=declaration
@semantic_tokens.token range=main.ds#decorator type=decorator
@semantic_tokens.token range=main.ds#function type=function modifiers=declaration
```

## Lexical Tokens

### Leave lexical tokens to the grammar

Comments and scalar literals do not duplicate the grammar's lexical classifications.

```ds main.ds
// An ordinary comment.
"text";
42;
true;
```

```query semantic_tokens main.ds
@semantic_tokens.none
```
