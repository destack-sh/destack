# Arrow Function Arguments

## Arrow Functions as Arguments

### callback in method call

Single param callbacks get parentheses added.

```ds
array.map(x => x * 2)
```

```ds expected
array.map((x) => x * 2);
```

### callback with parens

Parentheses are preserved when already present.

```ds
array.map((x) => x * 2)
```

```ds expected
array.map((x) => x * 2);
```

### callback with block body

Block bodies in callbacks expand to multiple lines.

```ds
array.forEach(item => { console.log(item) })
```

```ds expected
array.forEach((item) => {
    console.log(item)
});
```

### multiple callbacks

Each callback in a chain gets parentheses.

```ds
array.filter(x => x > 0).map(x => x * 2)
```

```ds expected
array.filter((x) => x > 0).map((x) => x * 2);
```

### callback with complex expression

Multi-param callbacks work with additional arguments.

```ds
array.reduce((acc, x) => acc + x, 0)
```

```ds expected
array.reduce((acc, x) => acc + x, 0);
```
