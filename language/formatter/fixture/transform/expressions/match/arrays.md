# Array Match Patterns

## Array Patterns

### array destructuring

Arrays can be destructured in match patterns.

```ds
match (arr) { [] => "empty"; [a] => `one: ${a}`; [a, b] => `two: ${a}, ${b}`; _ => "many" }
```

```ds expected
match (arr) {
    [] => "empty"
    [a] => `one: ${a}`
    [a, b] => `two: ${a}, ${b}`
    _ => "many"
}
```

### array with rest

Rest patterns capture remaining elements.

```ds
match (arr) { [first, ...rest] => first; [] => null }
```

```ds expected
match (arr) {
    [first, ...rest] => first
    [] => null
}
```

### array first and last

The `..` pattern matches elements in the middle.
Multi-element tuple results do not need a trailing comma.

```ds
match (arr) { [first, ..., last] => (first, last); _ => null }
```

```ds expected
match (arr) {
    [first, ..., last] => (first, last)
    _ => null
}
```
