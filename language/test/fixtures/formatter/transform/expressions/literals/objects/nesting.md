# Nested Objects

## Nested Objects

### nested object

Nested objects stay on one line if short.

```ds
const x = { a: { b: 1 } }
```

```ds expected
const x = { a: { b: 1 } };
```

### deeply nested object

Any depth of nesting is preserved if short.

```ds
const x = { a: { b: { c: { d: 1 } } } }
```

```ds expected
const x = { a: { b: { c: { d: 1 } } } };
```

### object with array value

Arrays can be object property values.

```ds
const x = { items: [1, 2, 3] }
```

```ds expected
const x = { items: [1, 2, 3] };
```
