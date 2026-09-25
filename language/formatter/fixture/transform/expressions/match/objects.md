# Object Match Patterns

## Object Patterns

### object destructuring

Objects can be destructured in match patterns.

```tspp
match (user) { { name: "admin" } => "admin user"; { name, age } => `${name} is ${age}`; _ => "unknown" }
```

```tspp expected
match (user) {
    { name: "admin" } => "admin user"
    { name, age } => `${name} is ${age}`
    _ => "unknown"
}
```

### struct pattern

Named struct patterns include the struct name.

```tspp
match (point) { Point { x: 0, y: 0 } => "origin"; Point { x, y } => `at (${x}, ${y})` }
```

```tspp expected
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at (${x}, ${y})`
}
```
