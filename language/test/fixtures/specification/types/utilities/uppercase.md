# Uppercase

`Uppercase` converts string literal types to uppercase.

### uppercase converts literals

```ts libs=es5
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";
```

### uppercase distributes over unions

```ts libs=es5
type Value = Uppercase<"yes" | "no">;

const ok: Value = "YES";
const ok2: Value = "NO";
```

### uppercase rejects original casing

```ts libs=es5
type Value = Uppercase<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
