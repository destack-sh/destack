# Variant Match Patterns

## Variant Patterns

### enum variant

Enum variants can be matched with destructuring.

```ds
match (result) { Ok(value) => value; Err(e) => throw e }
```

```ds expected
match (result) {
    Ok(value) => value
    Err(e) => throw e
}
```

### qualified variant

Variants can be qualified with their enum name.

```ds
match (result) { Result.Ok(v) => v; Result.Err(e) => handle(e) }
```

```ds expected
match (result) {
    Result.Ok(v) => v
    Result.Err(e) => handle(e)
}
```

### option pattern

Option types use Some and None variants.

```ds
match (opt) { Some(x) => x; None => default }
```

```ds expected
match (opt) {
    Some(x) => x
    None => default
}
```
