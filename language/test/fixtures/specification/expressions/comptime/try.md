# Try

Comptime expressions use normal `try`, `catch`, `finally`, and `?` typing.

## typing

### comptime try catch joins branch types

> `try/catch` inside comptime has normal expression typing.

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

### comptime try finally keeps the body type

> `finally` does not change the expression value type.

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

### comptime try can catch propagated failures

> `?` inside a comptime try can transfer failure to the local catch.

```ds
function parse(value: int): Result<int, string> {
    if (value > 0) {
        Result.ok(value)
    } else {
        Result.err("bad")
    }
}

const value = comptime {
    try {
        parse(0)?
    } catch (error) {
        error satisfies string;
        1
    }
};

value satisfies int;
```

### comptime values enter runtime try propagation

> Runtime `?` can consume values produced by comptime expressions.

```ds
function parse(value: int): Result<int, string> {
    Result.ok(value)
}

function compute(value: int): Result<int, string> {
    const current = comptime {
        1
    };

    const parsed = parse(current)?;
    Result.ok(parsed + value)
}
```

## rejections

### comptime try propagation rejects runtime inputs

> Comptime `?` cannot depend on dynamic function parameters.

```ds
function parse(value: int): Result<int, string> {
    Result.ok(value)
}

function compute(value: int): int {
    const result = comptime {
        parse(value)?
    };

    result
}
```

- contains: static expression

### comptime try propagation needs an exit path

> Uncaught `?` cannot leave a comptime expression without a compatible result path.

```ds
function parse(value: int): Result<int, string> {
    Result.ok(value)
}

const value = comptime {
    parse(1)?
};

value satisfies int;
```

- contains: failure cannot leave through return type
