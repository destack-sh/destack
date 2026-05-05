# Capitalize

`Capitalize` uppercases the first character of a string literal type.

### capitalize converts first character

```ts libs=es5
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
```

### capitalize distributes over unions

```ts libs=es5
type Value = Capitalize<"yes" | "no">;

const ok: Value = "Yes";
const ok2: Value = "No";
```

### capitalize rejects original casing

```ts libs=es5
type Value = Capitalize<"hello">;

const bad: Value = "hello";
```

- contains: not assignable
