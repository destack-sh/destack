# Narrowing Flow

## alias writes and captures

### nested function writes invalidate prior local narrows

Calling a nested function that mutates a captured local invalidates the caller's previous local narrowing.

```ds
let value: string | null = "ok";

if (value !== null) {
    const clear = () => {
        value = null;
    };

    clear();
    value satisfies string;
}
```

- contains: not assignable

### nested alias writes invalidate discriminant member availability

A mutation through an object alias invalidates discriminant-based member availability on the original binding.

```ds
type Ready = { kind: "ready"; payload: string };
type Idle = { kind: "idle" };

let box: { state: Ready | Idle } = { state: { kind: "ready", payload: "ok" } };

if (box.state.kind == "ready") {
    const mutate = (target: { state: Ready | Idle }) => {
        target.state = { kind: "idle" };
    };

    mutate(box);
    box.state.payload;
}
```

- contains: does not exist

### nested write through helper return invalidates discriminant member availability

Mutations performed by returned helper closures invalidate previously narrowed discriminant members.

```ds
type Ready = { kind: "ready"; payload: string };
type Idle = { kind: "idle" };

let box: { state: Ready | Idle } = { state: { kind: "ready", payload: "ok" } };

if (box.state.kind == "ready") {
    const writer = (target: { state: Ready | Idle }) => () => {
        target.state = { kind: "idle" };
    };

    const invalidate = writer(box);
    invalidate();
    box.state.payload;
}
```

- contains: does not exist

## joins and loops

### loop closure reads observe joined widened state

Closures created before a loop mutation observe the joined post-loop type, not the pre-loop narrow.

```ds
let value: string | number = "ok";
const read = () => value;

while (true) {
    value = 1;
    break;
}

const current = read();
current satisfies string;
```

- contains: not assignable

### branch joins keep only shared member availability

After control-flow joins that assign different variants, only members shared by all variants remain accessible.

```ds
type A = { kind: "a"; payload: string };
type B = { kind: "b" };

let value: A | B = { kind: "a", payload: "ok" };
let take_first = true;

if (take_first) {
    value = { kind: "a", payload: "next" };
} else {
    value = { kind: "b" };
}

value.payload;
```

- contains: does not exist

### loop writes invalidate prior branch narrows after re entry

A loop iteration that writes a captured local invalidates narrows established earlier in that branch.

```ds
let value: "a" | "b" = "a";

while (true) {
    if (value == "a") {
        value satisfies "a";
        value = "b";
    }

    break;
}

value satisfies "a";
```

- contains: not assignable

## destructuring and property writes

### destructured aliases do not preserve stale property narrows after writes

Destructured aliases to the same object cannot preserve stale property narrows after any alias writes.

```ds
let state: { value: string | number } = { value: "ok" };
const alias = state;

if (state.value is string) {
    alias.value = 1;
    state.value.toUpperCase();
}
```

- contains: does not exist

### property writes invalidate narrowed optional reads

Once an optional property is reassigned, previous `!== undefined` narrowing for that property is no longer valid.

```ds
let box: { value?: string } = { value: "ok" };

if (box.value !== undefined) {
    box.value satisfies string;
    box.value = undefined;
    box.value satisfies string;
}
```

- contains: not assignable

### nested property writes invalidate prior dotted-name narrows

Nested helper writes to dotted properties invalidate earlier dotted-name narrowing at the call site.

```ds
let box: { inner: { value?: string } } = { inner: { value: "ok" } };

if (box.inner.value !== undefined) {
    const mutate = () => {
        box.inner.value = undefined;
    };

    mutate();
    box.inner.value satisfies string;
}
```

- contains: not assignable
