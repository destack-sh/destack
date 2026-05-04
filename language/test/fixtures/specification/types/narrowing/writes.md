# Writes

Narrowing is flow-sensitive.
Writes and joins remove facts that no longer hold.

## local writes

### assignment invalidates a prior null check narrow

> A local reassignment invalidates a previously established narrow.

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

> Reassigning a discriminated union value removes member access from the previous branch.

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

> Loop joins do not preserve one-iteration narrow facts outside the loop.

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

> Facts after a branch join include only what both branches guarantee.

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

## alias and member writes

### writes through aliases invalidate discriminant narrows

> Writes through an alias invalidate discriminant member availability on the original value.

```ds
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

let box: { state: Ready | Idle } = { state: { kind: "ready", payload: "ok" } };
let alias = box;

if (box.state.kind == "ready") {
    alias.state = { kind: "idle" };
    box.state.payload;
}
```

- contains: does not exist

### index writes invalidate prior tuple element narrows

> Writes through index expressions invalidate previously established tuple element narrows.

```ds
let pair: (string | null, int32) = ("ok", 1);

if (pair[0] != null) {
    pair[0] satisfies string;
    pair[0] = null;
    pair[0] satisfies string;
}
```

- contains: not assignable
