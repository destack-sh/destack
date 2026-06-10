# Comptime

`comptime` expressions evaluate during compilation.

## comptime expressions

### comptime expressions yield value types

Comptime expressions type check as the body type.

```ds
const value: int32 = comptime 1 + 2;
value satisfies int32;
```

### comptime expressions keep exact values

Literal comptime results remain exact static values.

```ds
const value = comptime 1 + 2;
value satisfies 3;
```

### comptime block yields last expression type

Comptime blocks evaluate to the final expression.

```ds
const value: int32 = comptime {
    let base = 3;
    base + 2
};
value satisfies int32;
```

### comptime expressions call functions with static inputs

Comptime expressions can call functions when all inputs are static.

```ds
function add(a: int, b: int): int {
    a + b
}

const value = comptime add(1, 2);
value satisfies 3;
```

### comptime expressions reject runtime inputs

Comptime expressions cannot depend on dynamic function parameters.

```ds
function compute(value: int): int {
    const result = comptime value + 1;
    result
}
```

- contains: static expression
