# Hover

## Functions

### Hover over function name

Hovering over a function reference should display its signature.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//          ^^^^^ use:greet
```

Hovering over `greet` at the call site should show "function greet".

```query hover use:greet
function greet
```

## Structs

### Hover over struct name

Hovering over a struct type reference should display its kind.

```ds
struct Point {
//     ^^^^^ def:Point
    x: float32,
    y: float32,
}

const p: Point = Point { x: 1, y: 2 };
//       ^^^^^ use:Point
```

Hovering over `Point` in the type annotation should show "struct Point".

```query hover use:Point
struct Point
```

## Classes

### Hover over class name

Hovering over a class type reference should display its kind.

```ds
class Animal {
//    ^^^^^^ def:Animal
    name: string
//  ^^^^ def:Animal.name
}

const a: Animal = new Animal();
//       ^^^^^^ use:Animal
```

Hovering over `Animal` in the type annotation should show "class Animal".

```query hover use:Animal
class Animal
```

## Class Fields

### Hover over class field definition

Hovering over a class field definition should show its type.

```ds
class Person {
    name: string
//  ^^^^ def:name
    age: int32
//  ^^^ def:age
}
```

Hovering over `name` should show field info.

```query hover def:name
field name
```

Hovering over `age` should show field info.

```query hover def:age
field age
```

## Struct Fields

### Hover over struct field definition

Hovering over a struct field should show its type.

```ds
struct Vector2 {
    x: float32,
//  ^ def:x
    y: float32,
//  ^ def:y
}
```

Hovering over `x` should show field info.

```query hover def:x
field x
```

## Methods

### Hover over method definition

Hovering over a method should show its signature.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
//  ^^^ def:add
        return a + b;
    }
}
```

Hovering over `add` should show method info.

```query hover def:add
method add
```

## Enums

### Hover over enum field

Hovering over an enum variant should show enum info.

```ds
enum Color {
    Red,
//  ^^^ def:Red
    Green,
//  ^^^^^ def:Green
    Blue,
}
```

Hovering over `Red` should show enum field info.

```query hover def:Red
enum field Red
```

## Parameters

### Hover over function parameter

Hovering over a function parameter should show its type.

```ds
function multiply(x: int32, y: int32): int32 {
//                ^ def:x_param
//                          ^ def:y_param
    return x * y;
}
```

Hovering over `x` parameter should show parameter info.

```query hover def:x_param
parameter x
```
