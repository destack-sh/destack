# Chain Layout

Chain layout fixtures cover member chains mixed with calls, constructors, indexes, and arguments.

## Member Chain Formatting

### short chain stays on one line

Short chains fit on one line.

```ds
obj.method().result
```

```ds expected
obj.method().result;
```

### property access chain

Property access chains stay on one line when possible.

```ds line-width=50
very.long.deeply.nested.property.access
```

```ds expected
very.long.deeply.nested.property.access;
```

### private member access

Private member access keeps the `#` prefix.

```ts:main.ts
const value = foo . #bar . baz
```

```ts expected
const value = foo.#bar.baz;
```

### method chain with arguments

Method chains with various argument lengths.

```ds line-width=40
array.filter((x) => x > 0).map((x) => x * 2).reduce((a, b) => a + b, 0)
```

```ds expected
array
    .filter((x) => x > 0)
    .map((x) => x * 2)
    .reduce((a, b) => a + b, 0);
```

### chain starting with call

Chain starting with a function call.

```ds line-width=35
getData().process().transform().result()
```

```ds expected
getData()
    .process()
    .transform()
    .result();
```

### chain with constructor

Chain starting with new expression.

```ds line-width=40
new Builder().setName("test").setAge(25).build()
```

```ds expected
new Builder()
    .setName("test")
    .setAge(25)
    .build();
```

### conditional in method argument

Ternary inside a chained method call.

```ds line-width=50
data.filter((x) => isValid ? x.active : x.pending).map((x) => x.id)
```

```ds expected
data.filter((x) =>
    isValid ? x.active : x.pending,
).map((x) => x.id);
```

### chain with array index

Member chain including array indexing.

```ds line-width=40
users[0].profile.settings.theme
```

```ds expected
users[0].profile.settings.theme;
```

### complex chain with index and call

Mix of property access, indexing, and method calls.
Chains break at member segments under narrow width.

```ds line-width=35
obj.items[0].getValue().transform()
```

```ds expected
obj.items[0]
    .getValue()
    .transform();
```
