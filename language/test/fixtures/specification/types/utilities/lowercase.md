# Lowercase

`Lowercase` converts string literal types to lowercase.

## strings

### lowercase converts literals

The literal transforms at the type level.

```ds
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";
```

### lowercase distributes over unions

Each arm converts.

```ds
type Value = Lowercase<"YES" | "NO">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### lowercase rejects original casing

The original literal is gone.

```ds
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";
```

- contains: not assignable
