# ReturnType

`ReturnType` extracts a function return type.

## cases

### returntype extracts return values

> Function return types are preserved.

```ts libs=es5
type Value = ReturnType<() => string>;

const ok: Value = "ready";
ok satisfies string;
```

### returntype rejects wrong values

> Extracted return types reject unrelated values.

```ts libs=es5
type Value = ReturnType<() => string>;

const bad: Value = 1;
```

- contains: not assignable

### returntype keeps unions

> Union return types stay unions.

```ts libs=es5
type Value = ReturnType<() => "a" | "b">;

const ok: Value = "a";
const ok2: Value = "b";
```
