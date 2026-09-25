# Match Guards

## Guards

### guard condition

Guard conditions keep their required parentheses.

```tspp
match (n) { x if (x > 0) => "positive"; x if (x < 0) => "negative"; _ => "zero" }
```

```tspp expected
match (n) {
    x if (x > 0) => "positive"
    x if (x < 0) => "negative"
    _ => "zero"
}
```

### complex guard

Guards can use any boolean expression.

```tspp
match (user) { User { age } if (age >= 18) => "adult"; User { age } if (age >= 13) => "teen"; _ => "child" }
```

```tspp expected
match (user) {
    User { age } if (age >= 18) => "adult"
    User { age } if (age >= 13) => "teen"
    _ => "child"
}
```

### guard with method call

Method calls work in guard conditions.

```tspp
match (x) { v if (v.isValid()) => process(v); _ => null }
```

```tspp expected
match (x) {
    v if (v.isValid()) => process(v)
    _ => null
}
```

### tuple guard comparison

Tuple pattern guards keep comparison expressions inside the guard.

```tspp
match (pair) { (_, count) if (count > 0) => count; _ => 0 }
```

```tspp expected
match (pair) {
    (_, count) if (count > 0) => count
    _ => 0
}
```
