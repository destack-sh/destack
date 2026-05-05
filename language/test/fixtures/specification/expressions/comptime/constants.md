# Comptime Constants

Comptime expressions can produce constants for later code.

## constants

### comptime values initialize const declarations

> Comptime constants can be used by later code.

```ds
const scale = comptime 3;

function apply(value: int32): int32 {
    value * scale
}

apply(4) satisfies int32;
```

### comptime values keep exact values

> Comptime constants preserve literal precision when possible.

```ds
const count = comptime 2 + 2;
count satisfies 4;
```

### comptime values appear in runtime expressions

> Comptime values can be used like ordinary constants.

```ds
const count = comptime 2 + 2;
const values = [1, 2, 3, 4];

values[count - 1] satisfies int32;
```

### comptime values set fixed array lengths

> Comptime constants can be used as fixed array lengths.

```ds
const width = comptime 4;
type Lane = [uint8; width];

const lane: Lane = [1, 2, 3, 4];
lane satisfies [uint8; 4];
```

### comptime values select static conditionals

> Comptime expressions can include static conditional logic.

```ds
const width = comptime (if (true) { 8 } else { 4 });
type Lane = [uint8; width];

const lane: Lane = [1, 2, 3, 4, 5, 6, 7, 8];
lane satisfies [uint8; 8];
```

### comptime values reject runtime dependencies

> Comptime values reject dynamic function parameters.

```ds
function width(value: int32): int32 {
    const result = comptime value + 1;
    result
}
```

- contains: static expression
