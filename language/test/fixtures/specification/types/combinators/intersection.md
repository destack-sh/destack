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
