# Uncapitalize

`Uncapitalize` lowercases the first character of a string literal type.

## cases

### uncapitalize converts first character

> The first character is converted to lowercase.

```ts libs=es5
type Value = Uncapitalize<"Hello">;

const ok: Value = "hello";
```

### uncapitalize distributes over unions

> Union members are converted independently.

```ts libs=es5
type Value = Uncapitalize<"Yes" | "No">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### uncapitalize rejects original casing

> The original casing is not preserved.

```ts libs=es5
type Value = Uncapitalize<"Hello">;

const bad: Value = "Hello";
```

- contains: not assignable
