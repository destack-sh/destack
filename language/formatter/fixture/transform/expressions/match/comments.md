# Match Comments

## Trivia Boundaries

### match comments around selector and arms

Comments around selectors and arms stay on the same syntax boundaries.

```tspp
match (
    // selected event
    event
) {
    // click arm
    Click(pos) /* click pattern */ if (
        // valid position
        pos.isValid()
    ) => /* click body */ handleClick(pos);
    // fallback arm
    _ => fallback()
}
```

```tspp expected
match (
    // selected event
    event
) {
    // click arm
    Click(pos) /* click pattern */ if (
        // valid position
        pos.isValid()
    ) => /* click body */ handleClick(pos)
    // fallback arm
    _ => fallback()
}
```

### nested pattern trivia

Nested pattern comments stay inside their pattern containers.

```tspp
match (value) { Result.Ok(Point { x: /* x */ x, y: /* y */ y }) => x + y; [first, /* middle */ ..., last] => first + last; _ => 0 }
```

```tspp expected
match (value) {
    Result.Ok(Point { x: /* x */ x, y: /* y */ y }) => x + y
    [first, /* middle */ ..., last] => first + last
    _ => 0
}
```

### newtype object patterns

Newtype object patterns keep aliases, defaults, rest fields, and guards structured.

```tspp
match (shape) { Shape.Point({ x, y: renamed = 0, ...rest }) if (renamed > 0) => x + renamed; Shape.Line({ start: Point { x, y }, end }) => x + y; _ => 0 }
```

```tspp expected
match (shape) {
    Shape.Point({ x, y: renamed = 0, ...rest }) if (renamed > 0) => x + renamed
    Shape.Line({
        start: Point { x, y },
        end,
    }) => x + y
    _ => 0
}
```

### tagged tuple patterns

Tagged tuple patterns keep defaults, rests, and nested object patterns.

```tspp
match (result) { Result.Ok(Point { x, y }, meta = defaultMeta) => x + y; Result.Err(error, ...context) => context.length; _ => 0 }
```

```tspp expected
match (result) {
    Result.Ok(Point { x, y }, meta = defaultMeta) => x + y
    Result.Err(error, ...context) => context.length
    _ => 0
}
```

### array pattern variants

Array patterns keep leading, middle, and trailing rest forms distinct.

```tspp
match (items) { [first, ..., last] => first + last; [head, ...tail] => tail.length; [] => 0 }
```

```tspp expected
match (items) {
    [first, ..., last] => first + last
    [head, ...tail] => tail.length
    [] => 0
}
```

### pattern boundary comments

Comments inside complex patterns stay on the pattern side of the arrow.

```tspp
match (result) { Result.Ok(/* point */ Point { x: /* x */ x, y }) => x + y; Result.Err(/* error */ error) => error.code }
```

```tspp expected
match (result) {
    Result.Ok(/* point */ Point { x: /* x */ x, y }) => x + y
    Result.Err(/* error */ error) => error.code
}
```

### guard boundary comments

Comments around match guards stay between the pattern and branch body.

```tspp
match (packet) { Packet.Data(data) /* pattern */ if /* guard */ (data.isValid()) => /* body */ handle(data); _ => fallback() }
```

```tspp expected
match (packet) {
    Packet.Data(data) /* pattern */ if (/* guard */ data.isValid()) => /* body */ handle(data)
    _ => fallback()
}
```
