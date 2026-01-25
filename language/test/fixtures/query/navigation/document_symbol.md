# Document Symbol

## Basic Declarations

### Functions and structs

Document symbols should return all top-level declarations in a file for the outline view.

```ds
struct Point {
    x: float32,
    y: float32,
}

function add(a: int32, b: int32): int32 {
    return a + b;
}

class Animal {
    name: string
}
```

The file has 3 declarations: struct `Point`, function `add`, and class `Animal`.

```query document_symbols $0
Point
add
Animal
```

## Hierarchical Symbols

### Struct with fields

Struct members should appear as children of the struct.

```ds
struct Rectangle {
    width: float32,
    height: float32,

    function area(): float32 {
        return this.width * this.height;
    }
}
```

The struct has fields `width` and `height`, and a method `area`.

```query document_symbols $0
Rectangle
  width
  height
  area
```

### Class with members

Class members should appear as children of the class.

```ds
class Person {
    name: string
    age: int32

    function greet(): string {
        return "Hello, " + this.name;
    }
}
```

```query document_symbols $0
Person
  name
  age
  greet
```

### Enum with fields

Enum fields should appear as children of the enum.

```ds
enum Color {
    Red,
    Green,
    Blue,
}
```

```query document_symbols $0
Color
  Red
  Green
  Blue
```
