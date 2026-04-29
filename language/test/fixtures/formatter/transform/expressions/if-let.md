# If Let Expressions

Tests for `if let` expression formatting.

## Basic If Let

### if let expression

If let expressions keep spacing around `=` and format blocks.

```ds
const value = if (let Some(x) = maybe) { x } else { 0 }
```

```ds expected
const value = if (let Some(x) = maybe) { x } else { 0 };
```

### if let without else

If let without else still formats the block.

```ds
if (let (x, y) = point) { print(x + y) }
```

```ds expected
if (let (x, y) = point) {
    print(x + y);
}
```

### if let tagged object pattern

Tagged object patterns stay inside the parenthesized condition.

```ds
if (let Point { x, y } = value) { x + y } else { 0 }
```

```ds expected
if (let Point { x, y } = value) {
    x + y;
} else {
    0;
}
```

### if let else if chain

Else-if chains expand consistently when one branch expands.

```ds
if(let Some(value)=maybe){value}else if(let Err(error)=result){handle(error)}else{fallback()}
```

```ds expected
if (let Some(value) = maybe) {
    value;
} else if (let Err(error) = result) {
    handle(error);
} else {
    fallback();
}
```

### if let condition comments

Comments in the condition expand the whole head and branches.

```ds
const value = if (
    let Some(x) =
        // maybe value
        maybe
) { x } else { 0 }
```

```ds expected
const value = if (
    let Some(x) =
        // maybe value
        maybe
) {
    x
} else {
    0
};
```

### if let nested pattern comments

Comments in nested patterns stay with the pattern fields they describe.

```ds
if (let Result.Ok(Point { x: /* x */ x, y: /* y */ y }) = value) { x + y } else { 0 }
```

```ds expected
if (let Result.Ok(Point { x: /* x */ x, y: /* y */ y }) = value) {
    x + y;
} else {
    0;
}
```

### if let assignment boundary comments

Comments around the matched value stay on their side of the `=`.

```ds
const value = if (let Some(item) /* pattern */ = /* value */ maybe) { item } else { fallback }
```

```ds expected
const value = if (let Some(item) /* pattern */ = /* value */ maybe) { item } else { fallback };
```
