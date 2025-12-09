# Object Literals

Tests for object literal type inference and checking.

## Basic Objects

### object with properties

> Objects infer property types from their values.

```ds
const x = { a: 1, b: "two" }
```

### empty object

> Empty objects have no properties.

```ds
const x = {}
```

### nested object

> Objects can contain nested objects.

```ds
const x = { a: { b: 1 } }
```
