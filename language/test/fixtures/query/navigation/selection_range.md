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
4
```
