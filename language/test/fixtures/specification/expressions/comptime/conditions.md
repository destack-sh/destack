# Comptime Conditions

## comptime conditions

### comptime condition selects expression type

> Comptime conditions can drive normal if-expressions.

```ds
const value: int32 = if (comptime true) { 1 } else { 2 };
value satisfies int32;
```

### comptime condition still type checks both branches

> Both branches of a comptime condition must type check.

```ds
const value: int32 = if (comptime true) { 1 } else { "nope" };
```

- contains: is not assignable
