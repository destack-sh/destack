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

extension of Vector {
    magnitude(): float32 {
        return 0.0;
    }
}

interface Shape {
    area(): float;
}

enum Color {
    Red,
    Blue,
}

namespace math {
    function sum(a: int, b: int): int {
        return a + b;
    }
}

/*
multi line
comment
*/

// line comment block
// keeps going
// and going
```

This file has multi-line declarations that produce folding ranges: function, class, struct, interface, enum, namespace, and nested function.

```query folding_ranges $0
1-3
5-8
10-13
15-19
21-23
25-28
30-34
31-33
36-39
41-43
```

## Comments

### Comment blocks and mixed content

Multi-line comment blocks should be foldable.
Single line comments should not produce folding ranges.

```ds
/*
block
comment
*/

function keep() {}

// line comment block
// continues

const value = 1;

// single line comment

// another
// block
```

```query folding_ranges $0
1-4
8-9
15-16
```

### Ignore single line declarations

Single line declarations should not produce folding ranges.

```ds
function noop(): void {}
class Tiny {}
```

```query folding_ranges $0
<none>
```

## Damaged Syntax

### Keep folding ranges after malformed declarations

Folding ranges should still include later valid declarations after one malformed declaration.

```ds
function broken( {}

class Later {
    value: int32
}
```

```query folding_ranges $0
1-3
3-5
```
