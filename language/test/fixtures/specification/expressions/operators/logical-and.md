# Logical And

`&&` and `&&=` are not overloadable and short-circuit.

## expression

### logical and yields boolean for booleans

`&&` produces boolean when both operands are boolean.

```ds
let value: boolean = true && false;
```

## assignment

### logical and assignment preserves assignable values

`&&=` uses standard assignment rules.

```ds
type Value = { ok: true };

declare const fallback: Value;
let value: Value | undefined = fallback;

value &&= fallback;
value satisfies Value | undefined;
```

### logical and assignment rejects incompatible operands

`&&=` rejects incompatible right operands.

```ds
let value: string | undefined = "ok";

value &&= 1;
```

- contains: not assignable
