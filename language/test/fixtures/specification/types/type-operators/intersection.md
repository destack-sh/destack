# Intersection Types

Intersection types combine multiple shapes.

## object intersections

### intersection type combines object fields

> Intersection types require all fields.

```ds
type A = { a: number };
type B = { b: string };
type C = A & B;

let value: C = { a: 1, b: "ok" };
```

### intersection type rejects missing fields

> Missing fields in intersections are rejected.

```ds
type A = { a: number };
type B = { b: string };
type C = A & B;

let value: C = { a: 1 };
```

- contains: not assignable

### intersection type requires overlapping keys to satisfy both sides

> Overlapping keys in intersections must satisfy both member constraints.

```ds
type A = { value: number };
type B = { value: string };
type C = A & B;

let value: C = { value: "ok" };
```

- contains: not assignable

### intersection type preserves all overlapping-compatible keys

> Compatible overlapping keys are preserved in the intersection result.

```ds
type A = { value: number };
type B = { value: number, extra: string };
type C = A & B;

let value: C = { value: 1, extra: "ok" };
value.value satisfies number;
value.extra satisfies string;
```

### primitive intersections collapse to never-like behavior

> Incompatible primitive intersections reject concrete values.

```ds
type Both = string & int32;

let value: Both = "ok";
```

- contains: not assignable

### intersections keep stricter overlapping member constraints

> Overlapping intersection members keep the stricter shared constraint.

```ds
type A = { value: string | int32 };
type B = { value: string };
type C = A & B;

let value: C = { value: "ok" };
value.value satisfies string;
```
