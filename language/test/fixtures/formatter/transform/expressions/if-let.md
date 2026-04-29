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
    print(x + y)
}
```

### if let tagged object pattern

Tagged object patterns stay inside the parenthesized condition.

```ds
if (let Point { x, y } = value) { x + y } else { 0 }
```

```ds expected
if (let Point { x, y } = value) {
    x + y
} else {
    0
}
```

### if let else if chain

Else-if chains expand consistently when one branch expands.

```ds
if(let Some(value)=maybe){value}else if(let Err(error)=result){handle(error)}else{fallback()}
```

```ds expected
if (let Some(value) = maybe) {
    value
} else if (let Err(error) = result) {
    handle(error)
} else {
    fallback()
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
    x + y
} else {
    0
}
```

### if let nested tagged object pattern

Nested tagged object patterns break inside the parenthesized condition when needed.

```ds line-width=80
if (let Shape.Line { start: Point { x, y }, end } = shape) { x + y } else { 0 }
```

```ds expected
if (
    let Shape.Line {
        start: Point { x, y },
        end,
    } = shape
) {
    x + y
} else {
    0
}
```

### if let array boundary pattern

Array patterns keep omitted rest boundaries in conditions.

```ds
if (let [first, ..., last] = items) { use(first, last) } else { reset() }
```

```ds expected
if (let [first, ..., last] = items) {
    use(first, last)
} else {
    reset()
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

### if let function tail expression

If let expressions in function tail position preserve branch values.

```ds
function unwrap(maybe: Maybe<number>): number { if (let Some(value) = maybe) { value } else { 0 } }
```

```ds expected
function unwrap(maybe: Maybe<number>): number {
    if (let Some(value) = maybe) {
        value
    } else {
        0
    }
}
```

### if let method tail expression

If let expressions in method tail position preserve branch values.

```ds
class Box { value(): number { if (let Some(value) = this.cached) { value } else { this.compute() } } }
```

```ds expected
class Box {
    value(): number {
        if (let Some(value) = this.cached) {
            value
        } else {
            this.compute()
        }
    }
}
```

### if let void method tail

Void method bodies keep branch tail expressions semicolonless unless the semicolon was explicit.

```ds
class Box { apply(): void { if (let Some(value) = this.cached) { use(value) } else { reset() } } }
```

```ds expected
class Box {
    apply(): void {
        if (let Some(value) = this.cached) {
            use(value)
        } else {
            reset()
        }
    }
}
```

### if let tail with explicit branch statements

Explicit semicolons inside value-tail branches are preserved.

```ds
function unwrap(maybe: Maybe<number>): number { if (let Some(value) = maybe) { value; } else { 0 } }
```

```ds expected
function unwrap(maybe: Maybe<number>): number {
    if (let Some(value) = maybe) {
        value;
    } else {
        0
    }
}
```

### if let tail branch comments

Branch comments keep the value-producing tail expression semicolonless.

```ds
function unwrap(maybe: Maybe<number>): number { if (let Some(value) = maybe) { // present
value } else { // missing
0 } }
```

```ds expected
function unwrap(maybe: Maybe<number>): number {
    if (let Some(value) = maybe) {
        // present
        value
    } else {
        // missing
        0
    }
}
```

### if let nested match tail

Nested matches inside if-let tails preserve arm values.

```ds
function unwrap(maybe: Maybe<Result<number, Error>>): number { if (let Some(result) = maybe) { match (result) { Ok(value) => value; Err(_) => 0 } } else { 0 } }
```

```ds expected
function unwrap(maybe: Maybe<Result<number, Error>>): number {
    if (let Some(result) = maybe) {
        match (result) {
            Ok(value) => value
            Err(_) => 0
        }
    } else {
        0
    }
}
```
