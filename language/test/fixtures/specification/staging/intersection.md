# Intersection Types

Tests for intersection type assignability.

## Basic Intersections

### _merge object fields

> Intersection types merge object fields.

```ds
type A = { a: number };
type B = { b: string };
type C = A & B;
const value: C = { a: 1, b: "two" };
value satisfies { a: number, b: string };
```
