# Tuple Match Patterns

## Tuple Patterns

### tuple destructuring

Tuples can be destructured in match patterns.

```ds
match (point) { (0, 0) => "origin"; (x, 0) => `x-axis at ${x}`; (0, y) => `y-axis at ${y}`; (x, y) => `at (${x}, ${y})` }
```

```ds expected
match (point) {
    (0, 0) => "origin"
    (x, 0) => `x-axis at ${x}`
    (0, y) => `y-axis at ${y}`
    (x, y) => `at (${x}, ${y})`
}
```

### nested tuple

Tuple patterns can be nested.

```ds
match (data) { ((a, b), c) => a + b + c }
```

```ds expected
match (data) {
    ((a, b), c) => a + b + c
}
```
