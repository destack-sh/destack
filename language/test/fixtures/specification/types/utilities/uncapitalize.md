# Uncapitalize

`Uncapitalize` lowercases the first character of a string literal type.

## strings

### uncapitalize converts first character

Only the first character changes.

```ds
type Value = Uncapitalize<"Hello">;

const ok: Value = "hello";
```

### uncapitalize distributes over unions

Each arm converts.

```ds
type Value = Uncapitalize<"Yes" | "No">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### uncapitalize rejects original casing

The original literal is gone.

```ds
type Value = Uncapitalize<"Hello">;

const bad: Value = "Hello";
```

- contains: not assignable
