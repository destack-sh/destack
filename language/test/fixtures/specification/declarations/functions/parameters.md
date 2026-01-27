# Dynamic Parameters

Tests for dynamic parameter passing and defaults.

## dynamic parameters

### positional arguments map to parameters

> Positional arguments flow into parameters by order.

```ds
function sum(a: number, b: number): number {
    return a + b;
}

const value = sum(1, 2);
value satisfies number;
```

### defaulted parameters are optional

> Parameters with defaults can be omitted at call sites.

```ds
function greet(name: string = "hi"): string {
    return name;
}

greet() satisfies string;
greet("hello") satisfies string;
```

### defaulted parameter types are enforced

> Arguments still satisfy the declared parameter type.

```ds
function repeat(value: string = "hi"): string {
    return value;
}

repeat("ok");
repeat(1);
```

- contains: is not assignable
