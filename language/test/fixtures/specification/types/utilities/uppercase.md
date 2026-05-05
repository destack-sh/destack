# Uppercase

`Uppercase` converts string literal types to uppercase.

## cases

### uppercase converts literals

> String literals are converted to uppercase.

```ts libs=es5
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";
```

### uppercase distributes over unions

> Union members are converted independently.

```ts libs=es5
type Value = Uppercase<"yes" | "no">;

const ok: Value = "YES";
const ok2: Value = "NO";
```

### uppercase rejects original casing

> The original casing is not preserved.

```ts libs=es5
type Value = Uppercase<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
