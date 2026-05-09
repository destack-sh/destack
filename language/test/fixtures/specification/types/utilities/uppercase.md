# Uppercase

`Uppercase` converts string literal types to uppercase.

## strings

### uppercase converts literals

```ds
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";
```

### uppercase distributes over unions

```ds
type Value = Uppercase<"yes" | "no">;

const ok: Value = "YES";
const ok2: Value = "NO";
```

### uppercase rejects original casing

```ds
type Value = Uppercase<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
