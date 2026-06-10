# Discriminant Narrowing

Discriminant guard narrowing.

## equality guards

### discriminant guard narrows to matching variant

A literal field test picks the matching arm.

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

Bracket access tests the same field.

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

A failed test picks the other arms.

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

An optional discriminant cannot exclude its arm.

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

Aliases do not hide the discriminant.

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
