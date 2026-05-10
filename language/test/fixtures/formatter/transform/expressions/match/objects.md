# Object Match Patterns

## Object Patterns

### object destructuring

Objects can be destructured in match patterns.

```ds
match (user) { { name: "admin" } => "admin user"; { name, age } => `${name} is ${age}`; _ => "unknown" }
```

```ds expected
match (user) {
    { name: "admin" } => "admin user"
    { name, age } => `${name} is ${age}`
    _ => "unknown"
}
```

### struct pattern

Named struct patterns include the struct name.

```ds
match (point) { Point { x: 0, y: 0 } => "origin"; Point { x, y } => `at (${x}, ${y})` }
```

```ds expected
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at (${x}, ${y})`
}
```
