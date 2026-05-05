# Lowercase

`Lowercase` converts string literal types to lowercase.

### lowercase converts literals

```ts libs=es5
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";
```

### lowercase distributes over unions

```ts libs=es5
type Value = Lowercase<"YES" | "NO">;

const ok: Value = "yes";
const ok2: Value = "no";
```

### lowercase rejects original casing

```ts libs=es5
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";
```

- contains: not assignable
