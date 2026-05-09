# Capitalize

`Capitalize` uppercases the first character of a string literal type.

## strings

### capitalize converts first character

```ds
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
```

### capitalize distributes over unions

```ds
type Value = Capitalize<"yes" | "no">;

const ok: Value = "Yes";
const ok2: Value = "No";
```

### capitalize rejects original casing

```ds
type Value = Capitalize<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
