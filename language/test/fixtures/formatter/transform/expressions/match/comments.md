# Match Comments

## Trivia Boundaries

### match comments around selector and arms

Comments around selectors and arms stay on the same syntax boundaries.

```ds
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

```ds expected
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

```ds
match (value) { Result.Ok(Point { x: /* x */ x, y: /* y */ y }) => x + y; [first, /* middle */ ..., last] => first + last; _ => 0 }
```

```ds expected
match (value) {
    Result.Ok(Point { x: /* x */ x, y: /* y */ y }) => x + y
    [first, /* middle */ ..., last] => first + last
    _ => 0
}
```

### tagged object patterns

Tagged object patterns keep aliases, defaults, rest fields, and guards structured.

```ds
match (shape) { Shape.Point { x, y: renamed = 0, ...rest } if (renamed > 0) => x + renamed; Shape.Line { start: Point { x, y }, end } => x + y; _ => 0 }
```

```ds expected
match (shape) {
    Shape.Point { x, y: renamed = 0, ...rest } if (renamed > 0) => x + renamed
    Shape.Line {
        start: Point { x, y },
        end,
    } => x + y
    _ => 0
}
```

### tagged tuple patterns

Tagged tuple patterns keep defaults, rests, and nested object patterns.

```ds
match (result) { Result.Ok(Point { x, y }, meta = defaultMeta) => x + y; Result.Err(error, ...context) => context.length; _ => 0 }
```

```ds expected
match (result) {
    Result.Ok(Point { x, y }, meta = defaultMeta) => x + y
    Result.Err(error, ...context) => context.length
    _ => 0
}
```

### array pattern variants

Array patterns keep leading, middle, and trailing rest forms distinct.

```ds
match (items) { [first, ..., last] => first + last; [head, ...tail] => tail.length; [] => 0 }
```

```ds expected
match (items) {
    [first, ..., last] => first + last
    [head, ...tail] => tail.length
    [] => 0
}
```

### pattern boundary comments

Comments inside complex patterns stay on the pattern side of the arrow.

```ds
match (result) { Result.Ok(/* point */ Point { x: /* x */ x, y }) => x + y; Result.Err(/* error */ error) => error.code }
```

```ds expected
match (result) {
    Result.Ok(/* point */ Point { x: /* x */ x, y }) => x + y
    Result.Err(/* error */ error) => error.code
}
```
