# Match Guards

## Guards

### guard condition

Guard conditions keep their required parentheses.

```ds
match (n) { x if (x > 0) => "positive"; x if (x < 0) => "negative"; _ => "zero" }
```

```ds expected
match (n) {
    x if (x > 0) => "positive"
    x if (x < 0) => "negative"
    _ => "zero"
}
```

### complex guard

Guards can use any boolean expression.

```ds
match (user) { User { age } if (age >= 18) => "adult"; User { age } if (age >= 13) => "teen"; _ => "child" }
```

```ds expected
match (user) {
    User { age } if (age >= 18) => "adult"
    User { age } if (age >= 13) => "teen"
    _ => "child"
}
```

### guard with method call

Method calls work in guard conditions.

```ds
match (x) { v if (v.isValid()) => process(v); _ => null }
```

```ds expected
match (x) {
    v if (v.isValid()) => process(v)
    _ => null
}
```
