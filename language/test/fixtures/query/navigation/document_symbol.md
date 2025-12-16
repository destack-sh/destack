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
