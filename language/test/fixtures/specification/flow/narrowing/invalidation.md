# Narrowing Invalidation

Narrowing is flow-sensitive and should be invalidated by writes and joins.
These tests lock the invalidation boundaries so refined facts do not leak past mutation points.

## local writes

### assignment invalidates a prior null check narrow

> A local reassignment should invalidate a previously established narrow.

```ds
let value: string | null = "ok";

if (value != null) {
    value satisfies string;
    value = null;
    value satisfies string;
}
```

- contains: not assignable

### assignment invalidates discriminant member availability

> Reassigning a discriminated union value should invalidate member access from the previous branch.

```ds
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

let box: { state: Ready | Idle } = { state: { kind: "ready", payload: "ok" } };

if (box.state.kind == "ready") {
    box.state.payload satisfies string;
    box.state = { kind: "idle" };
}

box.state.payload;
```

- contains: does not exist

## loops and joins

### loop back-edges rejoin and widen narrowed locals

> Loop joins should not preserve one-iteration narrow facts outside the loop.

```ds
let value: "a" | "b" = "a";

while (true) {
    if (value == "a") {
        value = "b";
        break;
    }

    break;
}

value satisfies "a";
```

- contains: not assignable

### branch joins preserve only intersection of branch guarantees

> Facts after a branch join should only include what both branches guarantee.

```ds
let value: string | int32 = "ok";

if (true) {
    value = "next";
} else {
    value = 1;
}

value satisfies string;
```

- contains: not assignable

## closure capture

### captured locals observe post-write widened state

> Closures should observe the current variable state, not a stale narrowed snapshot.

```ds
let value: string | null = "ok";

if (value != null) {
    const read = () => value;
    value = null;
    read() satisfies string;
}
```

- contains: not assignable

### closure-returned values keep union contracts after mutation

> Closure return types should remain widened after captured variable mutation.

```ds
let value: string | null = "ok";
const read = () => value;

value = null;

const current = read();
current satisfies string;
```

- contains: not assignable
