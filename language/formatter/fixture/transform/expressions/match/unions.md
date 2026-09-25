# Union Match Patterns

## Union Patterns

### or pattern

Multiple patterns can be combined with `|`.

```tspp
match (x) { 1 | 2 | 3 => "small"; _ => "other" }
```

```tspp expected
match (x) {
    1 | 2 | 3 => "small"
    _ => "other"
}
```

### or pattern with strings

Or patterns work with any pattern type.

```tspp
match (s) { "a" | "b" | "c" => true; _ => false }
```

```tspp expected
match (s) {
    "a" | "b" | "c" => true
    _ => false
}
```


## Numeric Union Patterns

### numeric unions

Union patterns can match multiple literal values in one arm.

```tspp
match (n) { 1 | 2 | 3 => "small"; 4 | 5 => "medium"; _ => "large" }
```

```tspp expected
match (n) {
    1 | 2 | 3 => "small"
    4 | 5 => "medium"
    _ => "large"
}
```
