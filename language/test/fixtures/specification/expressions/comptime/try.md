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

### comptime try finally returns the body type

A comptime `try/finally` expression uses the body result type.

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

### nested comptime try blocks join locally

> Each nested `try/catch` joins its own body and catch results.

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

### nested comptime lambdas reject ? outside compatible returns

> `?` cannot propagate failure from a comptime block whose surrounding return type cannot carry it.

```ds
const value = comptime {
    const read = (): Result<int, Error> => {
        Result.ok(1)
    };

    read()?
};

value satisfies int;
```

- contains: failure cannot leave through return type
