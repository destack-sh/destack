# Comptime Try Nested Interaction

## nested try paths

### comptime nested try blocks preserve expression result types

> Nested comptime `try/catch` expressions should compose by joining each level's body and fallback results.

```ds
const value = comptime {
    try {
        try {
            1
        } catch error {
            2
        }
    } catch error {
        3
    }
};

value satisfies int;
```

### comptime try in result returning functions can feed try propagation

> A comptime-produced `Result` inside a `Result`-returning function should be consumable by `try`.

```ds
function parse(value: int): Result<int, Error> {
    Result.ok(value)
}

function compute(value: int): Result<int, Error> {
    const current = comptime {
        try {
            1
        } catch error {
            2
        }
    };

    const parsed = parse(current)?;
    Result.ok(parsed + value)
}
```

## runtime rejection

### nested comptime helpers reject runtime dependent try propagation

> A nested comptime helper must reject `try` propagation when any input is runtime-dependent.

```ds
function parse(value: int): Result<int, Error> {
    Result.ok(value)
}

function compute(value: int): int {
    const current = comptime {
        const read = () => parse(value)?;
        read()
    };

    current
}
```

- contains: static expression

### nested comptime helpers reject try unwrap outside try contexts

> `try` unwrap in nested comptime helpers must be rejected when the enclosing function is not `Try`-compatible.

```ds
const value = comptime {
    const read = (): Result<int, Error> => {
        Result.ok(1)
    };

    read()?
};

value satisfies int;
```

- contains: try unwrap requires a try return type
