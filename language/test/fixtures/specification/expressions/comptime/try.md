# Try

Comptime `try` uses static inputs and keeps ordinary `try/catch/finally` typing.

## typing

### comptime try catch returns union body type

In comptime `try/catch`, the expression type joins the body result with the catch fallback result.

```ds
const value = comptime {
    try {
        1
    } catch (error) {
        error satisfies unknown;
        "fallback"
    }
};

value satisfies int | string;
```

### comptime try finally keeps body type

A comptime `try/finally` expression keeps the body result type because `finally` is side-effect only.

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

- contains: static expression

## nested try

### comptime nested try blocks preserve expression result types

> Nested comptime `try/catch` expressions compose by joining each level's body and fallback results.

```ds
const value = comptime {
    try {
        try {
            1
        } catch (error) {
            2
        }
    } catch (error) {
        3
    }
};

value satisfies int;
```

### comptime try in result returning functions can feed try propagation

> A comptime-produced `Result` inside a `Result`-returning function is consumable by `try`.

```ds
function parse(value: int): Result<int, Error> {
    Result.ok(value)
}

function compute(value: int): Result<int, Error> {
    const current = comptime {
        try {
            1
        } catch (error) {
            2
        }
    };

    const parsed = parse(current)?;
    Result.ok(parsed + value)
}
```

## runtime inputs

### nested comptime lambdas reject runtime dependent try propagation

> A nested comptime lambda must reject `try` propagation when any input is runtime-dependent.

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

### nested comptime lambdas reject try unwrap outside try contexts

> `try` unwrap in nested comptime lambdas must be rejected when the enclosing function is not `Try`-compatible.

```ds
const value = comptime {
    const read = (): Result<int, Error> => {
        Result.ok(1)
    };

    read()?
};

value satisfies int;
```

- contains: try unwrap requires a Try return type
