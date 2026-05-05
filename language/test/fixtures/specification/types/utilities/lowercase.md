# Lowercase

`Lowercase` converts string literal types to lowercase.

## cases

### lowercase converts literals

> String literals are converted to lowercase.

```ts libs=es5
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";
```

### lowercase distributes over unions

> Union members are converted independently.

```ts libs=es5
type Value = Lowercase<"YES" | "NO">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### lowercase rejects original casing

> The original casing is not preserved.

```ts libs=es5
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";
```

- contains: not assignable
