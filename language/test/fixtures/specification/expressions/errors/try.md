# Try Operator

The `?` operator unwraps `Try` values and propagates failures when the enclosing return type accepts them.

## unwrap

### try unwrap uses Try branch

> The `?` operator unwraps Try values to their success type.

```ds
declare function getResult(): Result<int, Error>;

function read(): Result<int, Error> {
    const value = getResult()?;
    value satisfies int;
    return Result.ok(value);
}
```

### try unwrap accepts structural TryBranch

> The `?` operator accepts Try implementations with a compatible TryBranch shape.

```ds
type FancyBranch<T, E> =
    | { kind: "continue", value: T }
    | { kind: "failure", error: E };

struct FancyTry<T, E> {
    value: FancyBranch<T, E>;
}

extension<T, E> of FancyTry<T, E> implements Try {
    type Value = T;
    type Error = E;
    branch(): FancyBranch<T, E> {
        this.value
    }
}

declare function getFancy(): FancyTry<int, Error>;

function read(): Result<int, Error> {
    const value = getFancy()?;
    value satisfies int;
    return Result.ok(value);
}
```

### try unwrap accepts TryBranch aliases

> The `?` operator accepts Try.branch return types that alias TryBranch.

```ds
type AliasBranch<T, E> = TryBranch<T, E>;

struct AliasTry<T, E> {
    value: Result<T, E>;
}

extension<T, E> of AliasTry<T, E> implements Try {
    type Value = T;
    type Error = E;
    branch(): AliasBranch<T, E> {
        match (this.value) {
            Ok { value } => ({ kind: "continue", value })
            Err { error } => ({ kind: "failure", error })
        }
    }
}

declare function getAlias(): AliasTry<int, Error>;

function read(): Result<int, Error> {
    const value = getAlias()?;
    value satisfies int;
    return Result.ok(value);
}
```

### try failure must fit the return type

> A `?` failure that leaves the function must be accepted by the return type.

```ds
type BrokenBranch<T, E> =
    | { kind: "continue", value: T }
    | { kind: "failure", error: E };

struct BrokenTry<T, E> {
    value: BrokenBranch<T, E>;
}

extension<T, E> of BrokenTry<T, E> implements Try {
    type Value = T;
    type Error = E;
    branch(): BrokenBranch<T, E> {
        this.value
    }
}

declare function getBroken(): BrokenTry<int, Error>;

function read(): BrokenTry<int, Error> {
    const value = getBroken()?;
    return BrokenTry { value: { kind: "continue", value } };
}
```

- contains: FromFailure

### try unwrap rejects types without Try implementations

> The `?` operator requires the receiver to implement Try.

```ds
type LooseBranch<T, E> =
    | { kind: "continue", value: T }
    | { kind: "failure", error: E };

struct LooseTry<T, E> {
    value: LooseBranch<T, E>;
}

extension<T, E> of LooseTry<T, E> {
    branch(): LooseBranch<T, E> {
        this.value
    }
}

declare function getLoose(): LooseTry<int, Error>;

function read(): Result<int, Error> {
    const value = getLoose()?;
    return Result.ok(value);
}
```

- contains: no matching overload

### try unwrap rejects malformed TryBranch

> The `?` operator requires Try.branch to return TryBranch.

```ds
type BadBranch<T, E> =
    | { value: T }
    | { error: E };

struct BadTry<T, E> {
    value: BadBranch<T, E>;
}

extension<T, E> of BadTry<T, E> implements Try {
    type Value = T;
    type Error = E;
    branch(): BadBranch<T, E> {
        this.value
    }
}

declare function getBad(): BadTry<int, Error>;

function read(): Result<int, Error> {
    const value = getBad()?;
    return Result.ok(value);
}
```

- contains: Try.branch must return TryBranch

### try unwrap rejects wrong branch kinds

> The `?` operator rejects branches with incorrect kind discriminators.

```ds
type WrongBranch<T, E> =
    | { kind: "continue", value: T }
    | { kind: "bad", error: E };

struct WrongTry<T, E> {
    value: WrongBranch<T, E>;
}

extension<T, E> of WrongTry<T, E> implements Try {
    type Value = T;
    type Error = E;
    branch(): WrongBranch<T, E> {
        this.value
    }
}

declare function getWrong(): WrongTry<int, Error>;

function read(): Result<int, Error> {
    const value = getWrong()?;
    return Result.ok(value);
}
```

- contains: Try.branch must return TryBranch

### try unwrap preserves nullish success values

> The `?` operator does not remove nullish from the success value.

```ds
declare function getResult(): Result<int | null, Error>;

function read(): Result<int | null, Error> {
    const value = getResult()?;
    value satisfies int | null;
    return Result.ok(value);
}
```

### try unwrap unwraps only one Try layer

> The `?` operator unwraps a single Try layer.

```ds
declare function getNested(): Result<Result<int, Error>, Error>;

function read(): Result<Result<int, Error>, Error> {
    const value = getNested()?;
    value satisfies Result<int, Error>;
    return Result.ok(value);
}
```

### try unwrap merges error types across unions

> The `?` operator merges error types across Try unions.

```ds
struct MissingError implements Error {
    message: string;
}

struct BadError implements Error {
    message: string;
}

declare function getResult(): Result<int, MissingError> | Result<string, BadError>;

function read(): Result<int | string, MissingError | BadError> {
    const value = getResult()?;
    value satisfies int | string;
    return Result.ok(value);
}
```

### try unwrap rejects non Try values

> The `?` operator requires a Try implementation.

```ds
function read(): Result<int, Error> {
    const value = "nope"?;
    return Result.ok(0);
}
```

- contains: no matching overload

### try unwrap rejects nullish unions

> The `?` operator does not accept nullish unions.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

function read(): Result<int, Error> {
    const value = getMaybeResult()?;
    return Result.ok(value);
}
```

- contains: no matching overload

### try unwrap rejects unions with non Try values

> The `?` operator rejects unions that contain non Try values.

```ds
declare function getResult(): Result<int, Error> | string;

function read(): Result<int, Error> {
    const value = getResult()?;
    return Result.ok(value);
}
```

- contains: no matching overload

### try failure rejects plain return types

> A `?` failure cannot leave through a return type that cannot represent it.

```ds
declare function getResult(): Result<int, Error>;

function read(): int {
    const value = getResult()?;
    return value;
}
```

- contains: FromFailure
