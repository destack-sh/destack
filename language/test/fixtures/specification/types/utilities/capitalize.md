# Capitalize

`Capitalize` uppercases the first character of a string literal type.

## strings

### capitalize converts first character

Only the first character changes.

```ds
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
```

### capitalize distributes over unions

Each arm converts.

```ds
type Value = Capitalize<"yes" | "no">;

const ok: Value = "Yes";
const ok2: Value = "No";
```

### capitalize rejects original casing

The original literal is gone.

```ds
type Value = Capitalize<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
