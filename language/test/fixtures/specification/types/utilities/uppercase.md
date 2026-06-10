# Uppercase

`Uppercase` converts string literal types to uppercase.

## strings

### uppercase converts literals

The literal transforms at the type level.

```ds
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";
```

### uppercase distributes over unions

Each arm converts.

```ds
type Value = Uppercase<"yes" | "no">;

const ok: Value = "YES";
const ok2: Value = "NO";
```

### uppercase rejects original casing

The original literal is gone.

```ds
type Value = Uppercase<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
