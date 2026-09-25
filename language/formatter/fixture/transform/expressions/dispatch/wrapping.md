# Chain Wrapping

Chain wrapping fixtures cover member chains mixed with calls, constructors, indexes, and arguments.

## Member Chain Formatting

### short chain stays on one line

Short chains fit on one line.

```tspp
obj.method().result
```

```tspp expected
obj.method().result;
```

### property access chain

Property access chains stay on one line when possible.

```tspp line-width=50
very.long.deeply.nested.property.access
```

```tspp expected
very.long.deeply.nested.property.access;
```

### method chain with arguments

Method chains with various argument lengths.

```tspp line-width=40
array.filter((x) => x > 0).map((x) => x * 2).reduce((a, b) => a + b, 0)
```

```tspp expected
array
    .filter((x) => x > 0)
    .map((x) => x * 2)
    .reduce((a, b) => a + b, 0);
```

### chain starting with call

Chain starting with a function call.

```tspp line-width=35
getData().process().transform().result()
```

```tspp expected
getData()
    .process()
    .transform()
    .result();
```

### chain with constructor

Chain starting with new expression.

```tspp line-width=40
new Builder().setName("test").setAge(25).build()
```

```tspp expected
new Builder()
    .setName("test")
    .setAge(25)
    .build();
```

### conditional in method argument

Ternary inside a chained method call.

```tspp line-width=50
data.filter((x) => isValid ? x.active : x.pending).map((x) => x.id)
```

```tspp expected
data.filter((x) =>
    isValid ? x.active : x.pending,
).map((x) => x.id);
```

### chain with array index

Member chain including array indexing.

```tspp line-width=40
users[0].profile.settings.theme
```

```tspp expected
users[0].profile.settings.theme;
```

### complex chain with index and call

Mix of property access, indexing, and method calls.
Chains break at member segments under narrow width.

```tspp line-width=35
obj.items[0].getValue().transform()
```

```tspp expected
obj.items[0]
    .getValue()
    .transform();
```
