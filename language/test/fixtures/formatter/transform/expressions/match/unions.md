# Union Match Patterns

## Union Patterns

### or pattern

Multiple patterns can be combined with `|`.

```ds
match (x) { 1 | 2 | 3 => "small"; _ => "other" }
```

```ds expected
match (x) {
    1 | 2 | 3 => "small"
    _ => "other"
}
```

### or pattern with strings

Or patterns work with any pattern type.

```ds
match (s) { "a" | "b" | "c" => true; _ => false }
```

```ds expected
match (s) {
    "a" | "b" | "c" => true
    _ => false
}
```


## Numeric Union Patterns

### numeric unions

Union patterns can match multiple literal values in one arm.

```ds
match (n) { 1 | 2 | 3 => "small"; 4 | 5 => "medium"; _ => "large" }
```

```ds expected
match (n) {
    1 | 2 | 3 => "small"
    4 | 5 => "medium"
    _ => "large"
}
```
