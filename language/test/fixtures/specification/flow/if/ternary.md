# Ternary Expressions

Tests for ternary expression typing.

## typing

### ternary yields union of branch types

> Ternary expressions produce the union of branch types.

```ds
const value = true ? 1 : "hi";
value satisfies int | string;
```

### ternary respects contextual type

> Contextual types constrain ternary branches.

```ds
const value: int = true ? 1 : 2;
value satisfies int;
```

### ternary rejects incompatible branch

> Branches must satisfy the contextual type.

```ds
const value: int = true ? 1 : "hi";
```

- contains: not assignable
