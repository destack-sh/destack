# Call Wrapping

## Line Breaking

### call breaks when exceeding line width

When arguments exceed line width, they break to multiple lines.

```tspp line-width=20
foo(aLongArg, anotherLongArg, thirdArg)
```

```tspp expected
foo(
    aLongArg,
    anotherLongArg,
    thirdArg,
);
```

### call with many short arguments breaks at line width

Many short arguments also break when exceeding line width.

```tspp line-width=30
foo(a, b, c, d, e, f, g, h, i, j)
```

```tspp expected
foo(
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    h,
    i,
    j,
);
```

### nested function calls

Nested calls are preserved without extra spacing.

```tspp
foo(bar(baz(x)))
```

```tspp expected
foo(bar(baz(x)));
```

### callback as last argument

Arrow function callbacks stay on one line if short.

```tspp
array.map((item) => item.value)
```

```tspp expected
array.map((item) => item.value);
```

### callback with body

Block bodies in callbacks expand to multiple lines.

```tspp
array.map((item) => { return item.value })
```

```tspp expected
array.map((item) => {
    return item.value;
});
```
