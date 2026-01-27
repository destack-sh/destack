# Selection Range

## Basic Expressions

### Selection depth in function body

The cursor is inside a nested expression. Expanding selection should move through: identifier -> expression -> statement -> function body -> function -> module.

```ds
fn add(a: int, b: int) -> int {
    return $0a + b;
}
```

The cursor is on `a` inside the function. The selection hierarchy includes the identifier, binary expression, return statement, function body, and function declaration.

```query selection_range $0
main.ds:2:12-2:13
main.ds:2:12-2:17
main.ds:2:5-2:17
main.ds:2:5-2:18
```

### Selection depth in nested arithmetic

The cursor is inside a nested arithmetic expression with parentheses.
Expanding selection should walk from the identifier to the parenthesized expression and the full return statement.

```ds
function compute(value: int32): int32 {
    return ($0value + 1) * 2;
}
```

The snapshot asserts the full selection chain from leaf to root.

```query selection_range $0
main.ds:2:13-2:18
main.ds:2:13-2:22
main.ds:2:12-2:23
main.ds:2:12-2:27
main.ds:2:5-2:27
main.ds:2:5-2:28
main.ds:1:39-3:2
main.ds:1:1-3:2
```
