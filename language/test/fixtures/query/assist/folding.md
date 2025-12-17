# Folding Ranges

## Basic Declarations

### Functions and classes

Multi-line declarations like functions, classes, and structs should be foldable.

```ds
function add(a: int, b: int): int {
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

This file has 3 multi-line declarations that produce folding ranges: function, class, and struct.

```query folding $0
3
```
