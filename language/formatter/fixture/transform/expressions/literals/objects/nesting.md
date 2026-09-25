# Nested Objects

## Nested Objects

### nested object

Nested objects stay on one line if short.

```tspp
const x = { a: { b: 1 } }
```

```tspp expected
const x = { a: { b: 1 } };
```

### deeply nested object

Any depth of nesting is preserved if short.

```tspp
const x = { a: { b: { c: { d: 1 } } } }
```

```tspp expected
const x = { a: { b: { c: { d: 1 } } } };
```

### object with array value

Arrays can be object property values.

```tspp
const x = { items: [1, 2, 3] }
```

```tspp expected
const x = { items: [1, 2, 3] };
```
