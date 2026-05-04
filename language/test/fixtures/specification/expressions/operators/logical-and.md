# Logical And

`&&` and `&&=` are not overloadable and short-circuit.

## expression

### logical and yields boolean for booleans

> `&&` produces boolean when both operands are boolean.

```ds
let value: boolean = true && false;
```

## assignment

### logical and assignment preserves assignable values

> `&&=` uses standard assignment rules.

```ts
type Value = { ok: true };

declare const fallback: Value;
let value: Value | undefined = fallback;

value &&= fallback;
value satisfies Value | undefined;
```

### logical and assignment rejects incompatible values

> `&&=` rejects incompatible right hand side values.

```ts
let value: string | undefined = "ok";

value &&= 1;
```

- contains: not assignable
