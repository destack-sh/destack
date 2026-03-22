# Logical Operators

Logical operators are not overloadable and short-circuit.

## logical operators

### logical and yields boolean for booleans

> Logical and produces boolean when both operands are boolean.

```ds
let value: boolean = true && false;
```

### logical or yields boolean for booleans

> Logical or produces boolean when both operands are boolean.

```ds
let value: boolean = true || false;
```

### logical not yields boolean

> Logical not produces boolean.

```ds
let value: boolean = !true;
```

### coalesce yields non-nullish type

> Nullish coalescing preserves non-nullish types.

```ds
let value: string = "hello" ?? "fallback";
```

### coalesce rejects incompatible target

> Nullish coalescing result must satisfy the target type.

```ds
let value: number = "hello" ?? 0;
```

- type "hello" is not assignable to type number