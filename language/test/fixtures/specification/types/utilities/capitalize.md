# Capitalize

`Capitalize` uppercases the first character of a string literal type.

## cases

### capitalize converts first character

> The first character is converted to uppercase.

```ts libs=es5
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
```

### capitalize distributes over unions

> Union members are converted independently.

```ts libs=es5
type Value = Capitalize<"yes" | "no">;

const ok: Value = "Yes";
const ok2: Value = "No";
```

### capitalize rejects original casing

> The original casing is not preserved.

```ts libs=es5
type Value = Capitalize<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
