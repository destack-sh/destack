# Comptime

## comptime expressions

### comptime expression yields value type

> Comptime expressions type check as the body type.

```ds
const value: int32 = comptime 1 + 2;
value satisfies int32;
```

### comptime expression infers type from body

> Comptime expressions infer types like normal expressions.

```ds
const value = comptime 1 + 2;
value satisfies int;
```

### comptime block yields last expression type

> Comptime blocks evaluate to the final expression.

```ds
const value: int32 = comptime {
    let base = 3;
    base + 2
};
value satisfies int32;
```

### comptime expressions can call functions

> Comptime expressions can call functions when all inputs are static.

```ds
function add(a: int, b: int): int {
    a + b
}

const value = comptime add(1, 2);
value satisfies int;
```

### comptime expressions reject runtime-only symbols

> Comptime expressions reject runtime-only symbols from function parameters.

```ds
function compute(value: int): int {
    const result = comptime value + 1;
    result
}
```

- contains: static expression
