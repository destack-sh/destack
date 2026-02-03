# Comptime Slots

Tests for using comptime results as fixed compile-time values.

## comptime values

### comptime values can drive const declarations

> Comptime expressions can initialize constants used by later code.

```ds
const scale = comptime 3;

function apply(value: int32): int32 {
    value * scale
}

apply(4) satisfies int32;
```

### comptime values can drive runtime expressions

> Comptime results can be used like ordinary constants.

```ds
const count = comptime 2 + 2;
const values = [1, 2, 3, 4];

values[count - 1] satisfies int32;
```
