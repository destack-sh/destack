# ReturnType

`ReturnType` extracts a function return type.

## functions

### ReturnType extracts return values

The return type comes out.

```ds
type Value = ReturnType<() => string>;

const ok: Value = "ready";
ok satisfies string;
```

### ReturnType rejects wrong values

The extracted type is exact.

```ds
type Value = ReturnType<() => string>;

const bad: Value = 1;
```

- contains: not assignable

### ReturnType keeps unions

Union returns extract as unions.

```ds
type Value = ReturnType<() => "a" | "b">;

const ok: Value = "a";
const ok2: Value = "b";
```
