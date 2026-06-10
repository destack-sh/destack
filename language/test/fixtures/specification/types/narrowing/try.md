# Try

Narrowing survives `try` control flow conservatively.

## catch joins

### catch assignments widen post-try variable types

A catch assignment rejoins with the try path.

```ds
let value: string | int32 = "ok";

try {
    value = "next";
} catch (error) {
    value = 1;
}

value satisfies string;
```

- contains: not assignable

### catch assignments remove variant-specific members

Discriminant narrowing from the try path does not leak past catch writes.

```ds
type Ready = { kind: "ready"; payload: string };
type Idle = { kind: "idle" };

let state: Ready | Idle = { kind: "ready", payload: "ok" };

try {
    if (state.kind == "ready") {
        state.payload satisfies string;
    }
} catch (error) {
    state = { kind: "idle" };
}

state.payload;
```

- contains: does not exist

### catch joins keep only shared members

After try and catch join, only members common to all paths remain available.

```ds
let value: string | number = "ok";

try {
    value = "next";
} catch (error) {
    value = 1;
}

value.toUpperCase();
```

- contains: does not exist

## finally writes

### finally writes invalidate prior narrows

Writes in finally run on all exits and invalidate prior narrowing facts.

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

- contains: not assignable

### finally alias writes invalidate prior dotted narrows

A write through an alias in finally invalidates earlier dotted-name narrows.

```ds
let box: { inner: { value?: string } } = { inner: { value: "ok" } };

if (box.inner.value !== undefined) {
    try {
        box.inner.value satisfies string;
    } finally {
        const alias = box.inner;
        alias.value = undefined;
    }

    box.inner.value satisfies string;
}
```

- contains: not assignable

### finally helper writes invalidate prior null checks

A helper called from finally can invalidate null checks established in try.

```ds
let value: string | null = "ok";

if (value !== null) {
    const clear = () => {
        value = null;
    };

    try {
        value satisfies string;
    } finally {
        clear();
    }

    value satisfies string;
}
```

- contains: not assignable

## loop writes

### loop closures invalidate discriminant member narrows

Closure writes inside loops invalidate discriminant member narrows.

```ds
type Ready = { kind: "ready"; payload: string };
type Idle = { kind: "idle" };

let state: Ready | Idle = { kind: "ready", payload: "ok" };

const clear = () => {
    state = { kind: "idle" };
};

while (true) {
    if (state.kind == "ready") {
        clear();
        state.payload;
    }

    break;
}
```

- contains: does not exist
