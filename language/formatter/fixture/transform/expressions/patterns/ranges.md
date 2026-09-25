# Range Patterns

Range patterns use tight operator spacing.

## bounds

### pattern range bounds

Range operators are attached to their bounds.

```tspp
const label = match (value) {
    0 .. 10 => "small"
    0 ..= 10 => "inclusive"
    0 .. => "from"
    .. 10 => "to"
    ..= 10 => "through"
    _ => "other"
}
```

```tspp expected
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

```tspp
const label = match (value) {
    MIN .. MAX => "local"
    Limits.Low ..= Limits.High => "shared"
    _ => "other"
}
```

```tspp expected
const label = match (value) {
    MIN..MAX => "local"
    Limits.Low..=Limits.High => "shared"
    _ => "other"
};
```

## alternatives

### range alternatives

Range patterns bind tighter than alternatives.

```tspp
const isEdge = match (value) {
    0 .. 10 | 90 ..= 99 => true
    _ => false
}
```

```tspp expected
const isEdge = match (value) {
    0..10 | 90..=99 => true
    _ => false
};
```

## comments

### range pattern boundary comments

Comments around range pattern operators keep readable operator boundaries.

```tspp
const label = match (value) {
    0 /* min */ ..= /* max */ 10 => "small"
    MIN /* low */ .. /* high */ MAX => "symbolic"
    20 .. /* open */ => "large"
    _ => "other"
}
```

```tspp expected
const label = match (value) {
    0 /* min */ ..= /* max */ 10 => "small"
    MIN /* low */ .. /* high */ MAX => "symbolic"
    20 .. /* open */ => "large"
    _ => "other"
};
```
