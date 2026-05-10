# Discriminant Narrowing

Discriminant guard narrowing.

## equality guards

### discriminant guard narrows to matching variant

```ds
type Result = { kind: "ok"; value: string } | { kind: "err"; error: string };

const result: Result = { kind: "ok", value: "done" };
if (result.kind == "ok") {
    result satisfies { kind: "ok"; value: string };
} else {
    result satisfies { kind: "err"; error: string };
}
```

### discriminant guard narrows with index access

```ds
type Result = { kind: "ok"; value: string } | { kind: "err"; error: string };

const result: Result = { kind: "ok", value: "done" };
if (result["kind"] == "ok") {
    result satisfies { kind: "ok"; value: string };
} else {
    result satisfies { kind: "err"; error: string };
}
```

### discriminant guard narrows on not equals

```ds
type Result = { kind: "ok"; value: string } | { kind: "err"; error: string };

const result: Result = { kind: "ok", value: "done" };
if (result.kind != "ok") {
    result satisfies { kind: "err"; error: string };
} else {
    result satisfies { kind: "ok"; value: string };
}
```

### discriminant guard keeps optional variants on the false branch

```ds
type Result = { kind?: "ok"; value: string } | { kind: "err"; error: string };

const result: Result = { kind: "ok", value: "done" };
if (result.kind == "ok") {
    result satisfies { kind?: "ok"; value: string };
} else {
    result satisfies { kind?: "ok"; value: string } | { kind: "err"; error: string };
}
```

### discriminant guard narrows through type aliases

```ds
type Ok = { kind: "ok"; value: string };
type Err = { kind: "err"; error: string };
type Result = Ok | Err;

const result: Result = { kind: "ok", value: "done" };
if (result.kind == "ok") {
    result satisfies Ok;
} else {
    result satisfies Err;
}
```
