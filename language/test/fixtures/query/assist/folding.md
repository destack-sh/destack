# Folding Ranges

## Basic Declarations

### Functions and classes

Multi-line declarations like functions, classes, and structs should be foldable.

```ds
fn add(a: int, b: int) -> int {
    return a + b;
}

class Point {
    x: int;
    y: int;
}

struct Vector {
    dx: float;
    dy: float;
}
```

This file has 2 multi-line declarations that produce folding ranges: a function and a class.

```query folding $0
2
```
