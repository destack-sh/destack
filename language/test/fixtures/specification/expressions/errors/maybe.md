# Maybe

Postfix `?` keeps the value or lets the failure leave.

## result

### ? opens Try success

`?` opens a successful Try value.

```ds
declare function read(): Result<int, Error>;

function main(): Result<int, Error> {
    const value = read()?;
    value satisfies int;
    return Result.ok(value);
}
```

### ? opens one Try layer

Only the outer Try layer is opened.

```ds
declare function read(): Result<Result<int, Error>, Error>;

function main(): Result<Result<int, Error>, Error> {
    const value = read()?;
    value satisfies Result<int, Error>;
    return Result.ok(value);
}
```

### ? propagates nullish success values

Nullish success values become failures.

```ds
declare function read(): Result<int | null | undefined, Error | null> | undefined;

function main(): Result<int, Error | null | undefined> {
    const value = read()?;
    value satisfies int;
    return Result.ok(value);
}
```

### ? propagates nullish values

Nullish values can leave through compatible nullable returns.

```ds
declare function read(): string | null;

function main(): string | null {
    const value = read()?;
    value satisfies string;
    return value;
}
```

### ? propagates nullable Results

Outer nullish values and Result failures propagate together.

```ds
declare function read(): Result<int, Error> | null | undefined;

function main(): Result<int, Error | null | undefined> {
    const value = read()?;
    value satisfies int;
    return Result.ok(value);
}
```

### ? joins Result unions

A union of Results unwraps to the union of success values and propagates the union of errors.

```ds
struct MissingError implements Error {
    name: "MissingError";
    message: string;
}

struct BadError implements Error {
    name: "BadError";
    message: string;
}

declare function read(): Result<int, MissingError> | Result<string, BadError>;

function main(): Result<int | string, MissingError | BadError> {
    const value = read()?;
    value satisfies int | string;
    return Result.ok(value);
}
```

## custom

### ? works with Try implementors

Any type that implements Try can be unwrapped.

```ds
struct Maybe<T, E> {
    branchValue: TryBranch<T, E>;
}

extension<T, E> of Maybe<T, E> implements Try {
    type Value = T;
    type Failure = E;

    static fromValue(value: T): Maybe<T, E> {
        Maybe { branchValue: TryContinue { kind: "continue", value } }
    }

    branch(): TryBranch<T, E> {
        this.branchValue
    }
}

declare function read(): Maybe<int, Error>;

function main(): Result<int, Error> {
    const value = read()?;
    value satisfies int;
    return Result.ok(value);
}
```

### ? needs a compatible return type

A failure cannot leave through a return type that cannot represent it.

```ds
declare function read(): Result<int, Error>;

function main(): int {
    const value = read()?;
    return value;
}
```

- contains: failure cannot leave through return type

## errors

### ? rejects plain values

The `?` operator needs a nullable or Try value.

```ds
function main(): Result<int, Error> {
    const value = "nope"?;
    return Result.ok(0);
}
```

- contains: nullable or Try

### mixed Try unions need narrowing

The `?` operator needs a nullable value, a Try value, or a narrowed union.

```ds
declare function read(): Result<int, Error> | string | null;

function main(): Result<int, Error> {
    const value = read()?;
    return Result.ok(value);
}
```

- contains: narrow
