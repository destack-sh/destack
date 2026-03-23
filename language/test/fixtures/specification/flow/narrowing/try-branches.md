# Try Branch Narrowing

## joins across try catch

### catch assignments widen post try variable type

A variable reassigned in `catch` should rejoin with the `try` path as a widened union.

```ds
let value: string | int32 = "ok";

try {
    value = "next";
} catch error {
    value = 1;
}

value satisfies string;
```

- type null is not assignable to type string

### discriminant narrowing does not leak across catch writes

If `catch` writes a different variant, discriminant narrowing from `try` must not leak past the join.

```ds
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

let state: Ready | Idle = { kind: "ready", payload: "ok" };

try {
    if (state.kind == "ready") {
        state.payload satisfies string;
    }
} catch error {
    state = { kind: "idle" };
}

state.payload;
```

- contains: does not exist

### finally writes invalidate prior narrows

Writes in `finally` run on all exits, so they must invalidate prior branch narrowing facts.

```ds
let value: string | null = "ok";

if (value != null) {
    try {
        value satisfies string;
    } finally {
        value = null;
    }

    value satisfies string;
}
```

- type null is not assignable to type string