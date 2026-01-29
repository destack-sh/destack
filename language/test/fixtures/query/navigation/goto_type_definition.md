# Goto Type Definition

Tests for LSP goto type definition functionality.

## Basic Types

### Variable with class type

Go to type definition should navigate from a variable to its type.

```ds
class MyClass {
//    ^^^^^^^ def:MyClass
    value: int32
}

const x: MyClass = MyClass { value: 42 };
//    ^ use:x
```

```query goto_type_definition use:x
def:MyClass
```

### Variable with struct type

Go to type definition should navigate from a variable to its struct type.

```ds
struct Point {
//     ^^^^^ def:Point
    x: float32,
    y: float32,
}

const p: Point = Point { x: 1.0, y: 2.0 };
//    ^ use:p
```

```query goto_type_definition use:p
def:Point
```

### Parameter with type

Go to type definition should work on function parameters.

```ds
struct Config {
//     ^^^^^^ def:Config
    enabled: bool
}

function process(cfg: Config): void {}
//               ^^^ use:cfg
```

```query goto_type_definition use:cfg
def:Config
```

## Type References

### Direct type reference

Go to type definition on a type should go to that type's definition.

```ds
class Animal {}
//    ^^^^^^ def:Animal

const x: Animal = Animal {};
//       ^^^^^^ use:Animal
```

```query goto_type_definition use:Animal
def:Animal
```

### Type alias reference

Go to type definition on a type alias should navigate to the alias declaration.

```ds
type UserId = int32;
//   ^^^^^^ def:UserId

const id: UserId = 1;
//        ^^^^^^ use:UserId
```

```query goto_type_definition use:UserId
def:UserId
```

### Re-exported type reference

Go to type definition should follow re-export chains to the original type.

```ds:types.ds
export type Thing = string;
//          ^^^^^ def:Thing_source
```

```ds:re_exports.ds
export type { Thing } from "./types.ds";
//            ^^^^^ def:Thing
```

```ds:main.ds
import type { Thing } from "./re_exports.ds";

const value: Thing = "ok";
//           ^^^^^ use:Thing
```

```query goto_type_definition use:Thing
def:Thing_source
```

### Enum type

Go to type definition should work with enum types.

```ds
enum Color {
//   ^^^^^ def:Color
    Red,
    Green,
    Blue,
}

const c: Color = Color.Red;
//    ^ use:c
```

```query goto_type_definition use:c
def:Color
```

## Primitive Types

### Primitive type variable

For variables with primitive types, goto type definition returns no result.

```ds
const x: int32 = 42;
//    ^ use:x
```

```query goto_type_definition use:x
<none>
```
