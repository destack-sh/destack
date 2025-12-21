# Object Literals

Tests for object literal type inference and checking.

## Basic Objects

### object with properties

> Objects infer property types from their values.

```ds
const x = { a: 1, b: "two" };
x satisfies { a: 1, b: "two" };
```

### empty object

> Empty objects have no properties.

```ds
const x = {};
x satisfies {};
```

### nested object

> Objects can contain nested objects.

```ds
const x = { a: { b: 1 } };
x satisfies { a: { b: 1 } };
```

## Contextual Objects

### contextual object literal

> Object literals use contextual types for property inference.

```ds
const value: { a: number, b: string } = { a: 1, b: "hi" };
value satisfies { a: number, b: string };
```
