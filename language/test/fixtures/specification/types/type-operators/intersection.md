# Intersection Types

Intersection types combine multiple shapes.

## object intersections

### intersection type combines object fields

`&` requires both shapes at once.

```ds
type A = { a: number };
type B = { b: string };
type C = A & B;

let value: C = { a: 1, b: "ok" };
```

### intersection type rejects missing fields

Every side's fields are required.

```ds
type A = { a: number };
type B = { b: string };
type C = A & B;

let value: C = { a: 1 };
```

- contains: not assignable

### intersection type requires overlapping keys to satisfy both sides

Shared keys intersect their types.

```ds
type A = { value: number };
type B = { value: string };
type C = A & B;

let value: C = { value: "ok" };
```

- contains: not assignable

### intersection type preserves all overlapping-compatible keys

Compatible overlaps just work.

```ds
type A = { value: number };
type B = { value: number; extra: string };
type C = A & B;

let value: C = { value: 1, extra: "ok" };
value.value satisfies number;
value.extra satisfies string;
```

### primitive intersections collapse to never

Disjoint primitives share no values.

```ds
type Both = string & int32;

let value: Both = "ok";
```

- contains: not assignable

### intersections keep stricter overlapping member constraints

The narrower side wins for shared keys.

```ds
type A = { value: string | int32 };
type B = { value: string };
type C = A & B;

let value: C = { value: "ok" };
value.value satisfies string;
```
