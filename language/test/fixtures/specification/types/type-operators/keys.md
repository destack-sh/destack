# Key Queries

Key queries such as `keyof` and mapped types use the actual instantiated type.
They do not shrink an argument back to its constraint.

## constraint shapes

### keyof does not collapse to constraint keys

> `keyof` reflects the instantiated type rather than the constraint shape.

```ds
type Keys<T extends { a: number }> = keyof T;

type Actual = Keys<{ a: number, b: string }>;

const key: Actual = "b";
key satisfies "a" | "b";
```

### mapped keys do not collapse to constraint keys

> Mapped types iterate over the keys of the instantiated type.

```ds
type Flags<T extends { a: number }> = { [K in keyof T]: boolean };

type Actual = Flags<{ a: number, b: string }>;

const ok: Actual = { a: true, b: false };
ok satisfies Actual;
```

## union keys

### keyof over unions uses shared apparent keys

> `keyof (A | B)` is the intersection of keys on all members.

```ds
type A = { a: number, shared: string };
type B = { b: number, shared: string };

type Keys = keyof (A | B);

const ok: Keys = "shared";
ok satisfies Keys;
```

### keyof over unions rejects non shared keys

> Non shared keys do not appear in `keyof (A | B)`.

```ds
type A = { a: number, shared: string };
type B = { b: number, shared: string };

type Keys = keyof (A | B);

const bad: Keys = "a";
```

- contains: not assignable

## utility types

### constrained pick honors instantiated keys

> Constrained generics still see keys from the instantiated type.

```ds libs=es5
type PickFrom<T extends { a: number }, K extends keyof T> = Pick<T, K>;

type Result = PickFrom<{ a: number, b: string }, "b">;

const ok: Result = { b: "x" };
ok satisfies Result;
```
