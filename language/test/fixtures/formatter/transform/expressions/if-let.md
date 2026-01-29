# If Let Expressions

Tests for `if let` expression formatting.

## Basic If Let

### if let expression

If let expressions keep spacing around `=` and format blocks.

```ds
const value = if let Some(x) = maybe { x } else { 0 }
```

```ds expected
const value = if let Some(x) = maybe { x } else { 0 };
```

### if let without else

If let without else still formats the block.

```ds
if let (x, y) = point { print(x + y) }
```

```ds expected
if let (x, y) = point {
    print(x + y)
}
```
