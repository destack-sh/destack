# If Let Expressions

If-let fixtures cover pattern conditions, branch tails, comments, and nested control flow.

## If-Let Forms

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

### if let dereference pattern

Dereference prefixes stay attached to the pattern head.

```ds
if (let *Point { x, y } = point) { x + y } else { 0 }
```

```ds expected
if (let *Point { x, y } = point) {
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

### if let nested newtype object pattern

Nested newtype object patterns break inside the parenthesized condition when needed.

```ds line-width=80
if (let Shape.Line({ start: Point { x, y }, end }) = shape) { x + y } else { 0 }
```

```ds expected
if (
    let Shape.Line({
        start: Point { x, y },
        end,
    }) = shape
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

### if let default value

If-let default values stay compact when they fit.

```ds
function configure(options = if (let Some(entry) = maybe) { entry.options } else { defaultOptions }) { apply(options) }
```

```ds expected
function configure(
    options = if (let Some(entry) = maybe) { entry.options } else { defaultOptions },
) {
    apply(options)
}
```

### if let argument value

If-let call argument values stay compact when they fit.

```ds
render(if (let Some(value) = maybe) { value.current } else { defaultValue })
```

```ds expected
render(if (let Some(value) = maybe) { value.current } else { defaultValue });
```

### if let collection values

If-let values keep required grouping in collection and spread positions.

```ds
const values = [if (let Some(value) = maybe) { value } else { fallback }, ...(if (let Some(items) = maybeItems) { items } else { [] })]
const envelope = { value: if (let Some(value) = maybe) { value } else { fallback } }
```

```ds expected
const values = [
    if (let Some(value) = maybe) { value } else { fallback },
    ...(if (let Some(items) = maybeItems) { items } else { [] }),
];
const envelope = { value: if (let Some(value) = maybe) { value } else { fallback } };
```

### if let template value

Compact if-let values stay inline inside template interpolations.

```ds
const label = `value: ${if (let Some(value) = maybe) { value } else { fallback }}`
```

```ds expected
const label = `value: ${if (let Some(value) = maybe) { value } else { fallback }}`;
```

### if let lambda body value

If-let lambda body values stay compact when they fit.

```ds
const choose = (entry) => if (let Some(value) = entry) { value.current } else { defaultValue }
```

```ds expected
const choose = (entry) => if (let Some(value) = entry) { value.current } else { defaultValue };
```

### long if let initializer value

Long if-let initializer values expand all branch blocks.

```ds line-width=80
const value = if (let Some(entry) = source.lookup(user.id)) { buildEntryView(entry, context.locale, context.timeZone) } else { buildFallbackView(context.locale, context.timeZone) }
```

```ds expected
const value = if (let Some(entry) = source.lookup(user.id)) {
    buildEntryView(entry, context.locale, context.timeZone)
} else {
    buildFallbackView(context.locale, context.timeZone)
};
```

### long if let chain receiver value

Long if-let chain receiver values expand all branch blocks.

```ds line-width=80
const result = (if (let Some(entry) = maybe) { createReadyBuilder(entry, context) } else { createPendingBuilder(context) }).build().finalize()
```

```ds expected
const result = (if (let Some(entry) = maybe) {
    createReadyBuilder(entry, context)
} else {
    createPendingBuilder(context)
})
    .build()
    .finalize();
```

### multiline if let initializer value

Manually broken if-let values keep all branch blocks expanded.

```ds
const value = if (let Some(entry) = maybe) {
    entry.value
} else { fallback }
```

```ds expected
const value = if (let Some(entry) = maybe) {
    entry.value
} else {
    fallback
};
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

Branch comments keep tail expressions semicolonless.

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
