# Logical Or

`||` and `||=` are not overloadable and short-circuit.

## expression

### logical or yields boolean for booleans

> `||` produces boolean when both operands are boolean.

```ds
let value: boolean = true || false;
```

## assignment

### logical or assignment accepts assignable values

> `||=` uses standard assignment rules.

```ts
let value: string | undefined = undefined;

value ||= "fallback";
value satisfies string | undefined;
```

### logical or assignment rejects incompatible values

> `||=` rejects incompatible right hand side values.

```ts
let value: string = "ok";

value ||= 1;
```

- type string | 1 is not assignable to type string | undefined
