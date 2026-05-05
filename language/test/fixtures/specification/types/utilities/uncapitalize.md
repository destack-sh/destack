# Uncapitalize

`Uncapitalize` lowercases the first character of a string literal type.

### uncapitalize converts first character

```ds
type Value = Uncapitalize<"Hello">;

const ok: Value = "hello";
```

### uncapitalize distributes over unions

```ds
type Value = Uncapitalize<"Yes" | "No">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### uncapitalize rejects original casing

```ds
type Value = Uncapitalize<"Hello">;

const bad: Value = "Hello";
```

- contains: not assignable
