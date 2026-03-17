# Narrowing CFG Invalidation

## try finally joins

### finally alias writes invalidate prior dotted narrows

> If a `finally` path mutates through an alias, earlier dotted-name narrows must be forgotten on exit.

```ts
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

### catch joins widen post try member availability

> When `catch` can replace the value shape, the post-`try` join only keeps members common to all paths.

```ts
let value: string | number = "ok";

try {
    value = "next";
} catch error {
    value = 1;
}

value.toUpperCase();
```

- contains: does not exist

## closure writes inside loops

### loop closures invalidate discriminant member narrows

> A closure write inside a loop body must kill discriminant-based narrowing observed before the write.

```ts
type Ready = { kind: "ready"; payload: string };
type Idle = { kind: "idle" };

let state: Ready | Idle = { kind: "ready", payload: "ok" };

const clear = () => {
    state = { kind: "idle" };
};

while (true) {
    if (state.kind === "ready") {
        clear();
        state.payload;
    }

    break;
}
```

- contains: does not exist

### finally helper writes invalidate null narrows

> A helper called from `finally` that writes `null` must invalidate non-null facts established in `try`.

```ts
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

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```
