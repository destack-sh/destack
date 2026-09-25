# Arrow Function Arguments

## Arrow Functions as Arguments

### callback in method call

Single param callbacks get parentheses added.

```tspp
array.map(x => x * 2)
```

```tspp expected
array.map((x) => x * 2);
```

### callback with parens

Parentheses are preserved when already present.

```tspp
array.map((x) => x * 2)
```

```tspp expected
array.map((x) => x * 2);
```

### callback with block body

Block bodies in callbacks expand to multiple lines.

```tspp
array.forEach(item => { console.log(item) })
```

```tspp expected
array.forEach((item) => {
    console.log(item)
});
```

### multiple callbacks

Each callback in a chain gets parentheses.

```tspp
array.filter(x => x > 0).map(x => x * 2)
```

```tspp expected
array.filter((x) => x > 0).map((x) => x * 2);
```

### callback with complex expression

Multi-param callbacks work with additional arguments.

```tspp
array.reduce((acc, x) => acc + x, 0)
```

```tspp expected
array.reduce((acc, x) => acc + x, 0);
```
