# Lowercase

`Lowercase` converts string literal types to lowercase.

## strings

### lowercase converts literals

```ds
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";
```

### lowercase distributes over unions

```ds
type Value = Lowercase<"YES" | "NO">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### lowercase rejects original casing

```ds
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";
```

- contains: not assignable
