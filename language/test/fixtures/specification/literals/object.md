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

## Object Spreads

### object spread adds fields

> Object literals incorporate spread fields.

```ds
const base = { a: 1, b: "two" };
const value = { ...base, c: true };
value satisfies { a: number, b: string, c: boolean };
```

### object spread overrides fields

> Later fields override earlier spread fields.

```ds
const base = { a: 1, b: 2 };
const value = { ...base, b: "two" };
value satisfies { a: number, b: string };
```

### object spread preserves unions

> Union spreads produce union object shapes.

```ds
const value: { a: number } | { b: string } = { a: 1 };
const merged = { ...value };
merged satisfies { a: number } | { b: string };
```

### object spread unwraps aliases

> Object spreads unwrap structural type aliases.

```ds
type Base = { a: number };
const base: Base = { a: 1 };
const value = { ...base, b: "two" };
value satisfies { a: number, b: string };
```

### object spread merges intersections

> Object spreads merge intersection shapes.

```ds
const base: { a: number } & { b: string } = { a: 1, b: "two" };
const value = { ...base, c: true };
value satisfies { a: number, b: string, c: boolean };
```

### object spread with any

> Any spreads preserve the any type.

```ds
const value: any = { a: 1 };
const merged = { ...value, b: "two" };
merged satisfies any;
```

## Contextual Objects

### contextual object literal

> Object literals use contextual types for property inference.

```ds
const value: { a: number, b: string } = { a: 1, b: "hi" };
value satisfies { a: number, b: string };
```

### contextual object literal mismatch

> Object literal properties must satisfy contextual field types.

```ds
const value: { a: number, b: string } = { a: 1, b: 2 };
```

- contains: type { a: number, b: 2 } is not assignable to type { a: number, b: string }

## Object Members

### object toString resolves

> Object literals expose Object prototype members.


```ds libs=es5
const value = { a: 1, b: "two" };
const text = value.toString();
text satisfies string;
```

### object hasOwnProperty resolves

> Object literals expose hasOwnProperty.


```ds libs=es5
const value = { a: 1 };
const result = value.hasOwnProperty("a");
result satisfies boolean;
```
