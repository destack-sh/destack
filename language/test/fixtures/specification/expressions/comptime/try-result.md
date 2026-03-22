# Comptime Try Result

## try and comptime interaction

### comptime try catch returns union body type

In comptime `try/catch`, the expression type should join the body result with the catch fallback result.

```ds
const value = comptime {
    try {
        1
    } catch error {
        error satisfies unknown;
        "fallback"
    }
};

value satisfies int | string;
```

### comptime try finally keeps body type

A comptime `try/finally` expression should keep the body result type because `finally` is side-effect only.

```ds
const value = comptime {
    try {
        1
    } finally {
        2
    }
};

value satisfies int;
```

### comptime try result propagation rejects runtime dependencies

Comptime `try` propagation must fail when the propagated value depends on runtime data.

```ds
function read(value: int): Result<int, Error> {
    Result.ok(value)
}

function compute(value: int): int {
    const result = comptime {
        read(value)?
    };

    result
}
```

- static expression
