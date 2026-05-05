# ReturnType

`ReturnType` extracts a function return type.

### ReturnType extracts return values

```ds
type Value = ReturnType<() => string>;

const ok: Value = "ready";
ok satisfies string;
```

### ReturnType rejects wrong values

```ds
type Value = ReturnType<() => string>;

const bad: Value = 1;
```

- contains: not assignable

### ReturnType keeps unions

```ds
type Value = ReturnType<() => "a" | "b">;

const ok: Value = "a";
const ok2: Value = "b";
```
