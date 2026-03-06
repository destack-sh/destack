# Folding Range

## Basic Declarations

### Fold multi-line declarations

Folding ranges should cover multi-line declarations and block comments.

```ds:main.ds
function add(a: int, b: int): int {
    return a + b;
}

class Point {
    x: int;
    y: int;
}

/*
block
comment
*/
```

```lsp folding_range
range=0:0-2:0

range=4:0-7:0
```
