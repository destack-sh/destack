# Range Patterns

Range patterns use tight operator spacing.

## bounds

### pattern range bounds

Range operators are attached to their bounds.

```ds
const label = match (value) {
    0 .. 10 => "small"
    0 ..= 10 => "inclusive"
    0 .. => "from"
    .. 10 => "to"
    ..= 10 => "through"
    _ => "other"
}
```

```ds expected
const label = match (value) {
    0..10 => "small"
    0..=10 => "inclusive"
    0.. => "from"
    ..10 => "to"
    ..=10 => "through"
    _ => "other"
};
```

## symbolic

### symbolic bounds

Identifier and path bounds keep member spacing.

```ds
const label = match (value) {
    MIN .. MAX => "local"
    Limits.Low ..= Limits.High => "shared"
    _ => "other"
}
```

```ds expected
const label = match (value) {
    MIN..MAX => "local"
    Limits.Low..=Limits.High => "shared"
    _ => "other"
};
```

## alternatives

### range alternatives

Range patterns bind tighter than alternatives.

```ds
const isEdge = match (value) {
    0 .. 10 | 90 ..= 99 => true
    _ => false
}
```

```ds expected
const isEdge = match (value) {
    0..10 | 90..=99 => true
    _ => false
};
```
