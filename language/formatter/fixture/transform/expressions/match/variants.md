# Variant Match Patterns

## Variant Patterns

### enum variant

Enum variants can be matched with destructuring.

```tspp
match (result) { Ok(value) => value; Err(e) => panic("unhandled error") }
```

```tspp expected
match (result) {
    Ok(value) => value
    Err(e) => panic("unhandled error")
}
```

### qualified variant

Variants can be qualified with their enum name.

```tspp
match (result) { Result.Ok(v) => v; Result.Err(e) => handle(e) }
```

```tspp expected
match (result) {
    Result.Ok(v) => v
    Result.Err(e) => handle(e)
}
```

### option pattern

Option types use Some and None variants.

```tspp
match (opt) { Some(x) => x; None => default }
```

```tspp expected
match (opt) {
    Some(x) => x
    None => default
}
```
