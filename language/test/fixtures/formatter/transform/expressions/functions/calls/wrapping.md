# Call Wrapping

## Line Breaking

### call breaks when exceeding line width

When arguments exceed line width, they break to multiple lines.

```ds line-width=20
foo(aLongArg, anotherLongArg, thirdArg)
```

```ds expected
foo(
    aLongArg,
    anotherLongArg,
    thirdArg,
);
```

### call with many short arguments breaks at line width

Many short arguments also break when exceeding line width.

```ds line-width=30
foo(a, b, c, d, e, f, g, h, i, j)
```

```ds expected
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

```ds
foo(bar(baz(x)))
```

```ds expected
foo(bar(baz(x)));
```

### callback as last argument

Arrow function callbacks stay on one line if short.

```ds
array.map((item) => item.value)
```

```ds expected
array.map((item) => item.value);
```

### callback with body

Block bodies in callbacks expand to multiple lines.

```ds
array.map((item) => { return item.value })
```

```ds expected
array.map((item) => {
    return item.value;
});
```
