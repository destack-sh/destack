# Array Match Patterns

## Array Patterns

### array destructuring

Arrays can be destructured in match patterns.

```tspp
match (arr) { [] => "empty"; [a] => `one: ${a}`; [a, b] => `two: ${a}, ${b}`; _ => "many" }
```

```tspp expected
match (arr) {
    [] => "empty"
    [a] => `one: ${a}`
    [a, b] => `two: ${a}, ${b}`
    _ => "many"
}
```

### array with rest

Rest patterns capture remaining elements.

```tspp
match (arr) { [first, ...rest] => first; [] => null }
```

```tspp expected
match (arr) {
    [first, ...rest] => first
    [] => null
}
```

### array first and last

The `..` pattern matches elements in the middle.
Multi-element tuple results do not need a trailing comma.

```tspp
match (arr) { [first, ..., last] => (first, last); _ => null }
```

```tspp expected
match (arr) {
    [first, ..., last] => (first, last)
    _ => null
}
```
