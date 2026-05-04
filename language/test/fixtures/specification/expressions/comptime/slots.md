# Comptime Slots

Using comptime results as fixed compile-time values.

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

### comptime values can drive fixed array lengths

> Comptime constants can be used as fixed array lengths.

```ds
const width = comptime 4;
type Lane = [uint8; width];

const lane: Lane = [1, 2, 3, 4];
lane satisfies [uint8; 4];
```

### comptime values can drive conditional static slots

> Comptime slot expressions can include static conditional logic.

```ds
const width = comptime (if (true) { 8 } else { 4 });
type Lane = [uint8; width];

const lane: Lane = [1, 2, 3, 4, 5, 6, 7, 8];
lane satisfies [uint8; 8];
```

### comptime slots reject runtime-only dependencies

> Comptime slots reject expressions that require runtime execution.

```ds
function runtime_width(): int32 {
    4
}

const width = comptime runtime_width();
```

- contains: static expression
