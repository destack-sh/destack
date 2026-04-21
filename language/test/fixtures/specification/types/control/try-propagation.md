# Result And Try

## propagation and coalescing

### try propagation preserves unioned success and error types

Applying `try` to `Result` unions should preserve both success and error alternatives through propagation.

```ds
struct MissingError implements Error {
    message: string;
}

struct BadError implements Error {
    message: string;
}

declare function get(): Result<int, MissingError> | Result<string, BadError>;

function read(): Result<int | string, MissingError | BadError> {
    const value = get()?;
    value satisfies int | string;
    return Result.ok(value);
}
```

### try unwrap removes one layer and preserves inner result

A single `try` should peel exactly one `Result` layer and keep any inner `Result` untouched.

```ds
declare function get(): Result<Result<int, Error>, Error>;

function read(): Result<Result<int, Error>, Error> {
    const value = get()?;
    value satisfies Result<int, Error>;
    return Result.ok(value);
}
```

### nullish coalesce after try keeps success value typing

When `try` is followed by `??`, the success branch type should remain intact while fallback type is joined.

```ds
declare function get(): Result<int | null, Error>;

function read(): Result<int, Error> {
    const maybe = get()?;
    const value = maybe ?? 0;
    value satisfies int;
    return Result.ok(value);
}
```

### try propagation requires fromError on custom Try types

Custom `Try` types must provide `fromError` so propagated errors can be reconstructed.

```ds
type Branch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct Custom<T, E> {
    value: Branch<T, E>;
}

extension<T, E> of Custom<T, E> implements Try<T, E> {
    branch(): Branch<T, E> {
        this.value
    }
}

declare function get(): Custom<int, Error>;

function read(): Result<int, Error> {
    const value = get()?;
    return Result.ok(value);
}
```

- fromError
