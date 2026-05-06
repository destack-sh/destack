# Logical Or

`||` and `||=` are not overloadable and short-circuit.

## expression

### logical or yields boolean for booleans

`||` produces boolean when both operands are boolean.

```ds
let value: boolean = true || false;
```

## assignment

### logical or assignment accepts assignable values

`||=` uses standard assignment rules.

```ds
let value: string | undefined = undefined;

value ||= "fallback";
value satisfies string | undefined;
```

### logical or assignment rejects incompatible operands

`||=` rejects incompatible right operands.

```ds
let value: string = "ok";

value ||= 1;
```

- contains: not assignable
