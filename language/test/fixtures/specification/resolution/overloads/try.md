# Try and Coalesce Operators

Tests for the Try operator and nullish coalescing.

## Try unwrap

### try unwrap uses Try branch

> The ? operator unwraps Try values to their success type.

```ds
declare function getResult(): Result<int, Error>;

function read(): Result<int, Error> {
    const value = getResult()?;
    value satisfies int;
    return Result.ok(value);
}
```

### try unwrap accepts structural TryBranch

> The ? operator accepts Try implementations with a compatible TryBranch shape.

```ds
type FancyBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct FancyTry<T, E> {
    value: FancyBranch<T, E>;
}

extension<T, E> of FancyTry<T, E> implements Try<T, E> {
    branch(): FancyBranch<T, E> {
        this.value
    }

    static fromError(error: E): FancyTry<T, E> {
        FancyTry { value: { kind: "err", error } }
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

> The ? operator accepts Try.branch return types that alias TryBranch.

```ds
type AliasBranch<T, E> = TryBranch<T, E>;

struct AliasTry<T, E> {
    value: Result<T, E>;
}

extension<T, E> of AliasTry<T, E> implements Try<T, E> {
    branch(): AliasBranch<T, E> {
        match (this.value) {
            Ok { value } => ({ kind: "ok", value })
            Err { error } => ({ kind: "err", error })
        }
    }

    static fromError(error: E): AliasTry<T, E> {
        AliasTry { value: Result.err(error) }
    }
}

declare function getAlias(): AliasTry<int, Error>;

function read(): Result<int, Error> {
    const value = getAlias()?;
    value satisfies int;
    return Result.ok(value);
}
```

### try unwrap requires fromError

> The ? operator requires Try.fromError to be implemented.

```ds
type BrokenBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct BrokenTry<T, E> {
    value: BrokenBranch<T, E>;
}

extension<T, E> of BrokenTry<T, E> implements Try<T, E> {
    branch(): BrokenBranch<T, E> {
        this.value
    }
}

declare function getBroken(): BrokenTry<int, Error>;

function read(): Result<int, Error> {
    const value = getBroken()?;
    return Result.ok(value);
}
```

- fromError

### try unwrap rejects types without Try implementations

> The ? operator requires the receiver to implement Try.

```ds
type LooseBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct LooseTry<T, E> {
    value: LooseBranch<T, E>;
}

extension<T, E> of LooseTry<T, E> {
    branch(): LooseBranch<T, E> {
        this.value
    }

    static fromError(error: E): LooseTry<T, E> {
        LooseTry { value: { kind: "err", error } }
    }
}

declare function getLoose(): LooseTry<int, Error>;

function read(): Result<int, Error> {
    const value = getLoose()?;
    return Result.ok(value);
}
```

- contains: no matching overload

### try unwrap rejects invalid TryBranch

> The ? operator requires Try.branch to return TryBranch.

```ds
type BadBranch<T, E> =
    | { value: T }
    | { error: E };

struct BadTry<T, E> {
    value: BadBranch<T, E>;
}

extension<T, E> of BadTry<T, E> implements Try<T, E> {
    branch(): BadBranch<T, E> {
        this.value
    }

    static fromError(error: E): BadTry<T, E> {
        BadTry { value: { error } }
    }
}

declare function getBad(): BadTry<int, Error>;

function read(): Result<int, Error> {
    const value = getBad()?;
    return Result.ok(value);
}
```

- Try.branch must return TryBranch

### try unwrap rejects wrong branch kinds

> The ? operator rejects branches with incorrect kind discriminators.

```ds
type WrongBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "bad", error: E };

struct WrongTry<T, E> {
    value: WrongBranch<T, E>;
}

extension<T, E> of WrongTry<T, E> implements Try<T, E> {
    branch(): WrongBranch<T, E> {
        this.value
    }

    static fromError(error: E): WrongTry<T, E> {
        WrongTry { value: { kind: "bad", error } }
    }
}

declare function getWrong(): WrongTry<int, Error>;

function read(): Result<int, Error> {
    const value = getWrong()?;
    return Result.ok(value);
}
```

- Try.branch must return TryBranch

### try unwrap preserves nullish success values

> The ? operator does not remove nullish from the success value.

```ds
declare function getResult(): Result<int | null, Error>;

function read(): Result<int | null, Error> {
    const value = getResult()?;
    value satisfies int | null;
    return Result.ok(value);
}
```

### try unwrap unwraps only one Try layer

> The ? operator unwraps a single Try layer.

```ds
declare function getNested(): Result<Result<int, Error>, Error>;

function read(): Result<Result<int, Error>, Error> {
    const value = getNested()?;
    value satisfies Result<int, Error>;
    return Result.ok(value);
}
```

### try unwrap merges error types across unions

> The ? operator merges error types across Try unions.

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

> The ? operator requires a Try implementation.

```ds
function read(): Result<int, Error> {
    const value = "nope"?;
    return Result.ok(0);
}
```

- contains: no matching overload

### try unwrap rejects nullish unions

> The ? operator does not accept nullish unions.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

function read(): Result<int, Error> {
    const value = getMaybeResult()?;
    return Result.ok(value);
}
```

- contains: no matching overload

### try unwrap rejects unions with non Try values

> The ? operator rejects unions that contain non Try values.

```ds
declare function getResult(): Result<int, Error> | string;

function read(): Result<int, Error> {
    const value = getResult()?;
    return Result.ok(value);
}
```

- contains: no matching overload

### try unwrap requires Try return type

> The ? operator requires a Try return type on the enclosing function.

```ds
declare function getResult(): Result<int, Error>;

function read(): int {
    const value = getResult()?;
    return value;
}
```

- try unwrap requires a Try return type

## Try coalesce

### try coalesce uses Try branch

> The ?? operator uses Try when the left value implements it.

```ds
declare function getResult(): Result<int, Error>;

const value = getResult() ?? 0;
value satisfies int;
```

### try coalesce allows missing fromError

> The ?? operator does not require Try.fromError.

```ds
type BrokenBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct BrokenTry<T, E> {
    value: BrokenBranch<T, E>;
}

extension<T, E> of BrokenTry<T, E> implements Try<T, E> {
    branch(): BrokenBranch<T, E> {
        this.value
    }
}

declare function getBroken(): BrokenTry<int, Error>;

const value = getBroken() ?? 0;
value satisfies int;
```

### try coalesce unwraps nullish and Try

> The ?? operator coalesces nullish before unwrapping Try.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce removes nested nullish values

> The ?? operator removes nullish after unwrapping one Try layer.

```ds
declare function getMaybeResult(): Result<int | null, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce preserves fallback type when it differs

> The ?? operator returns a union of the unwrapped left and the fallback.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

const value = getMaybeResult() ?? "fallback";
value satisfies int | string;
```

### try coalesce merges with non Try unions

> The ?? operator preserves non Try union members.

```ds
declare function getMaybeResult(): Result<int, Error> | string;

const value = getMaybeResult() ?? 0;
value satisfies int | string;
```

### try coalesce merges nullish and non Try unions

> The ?? operator removes nullish and keeps non Try members.

```ds
declare function getMaybeResult(): Result<int, Error> | null | string;

const value = getMaybeResult() ?? 0;
value satisfies int | string;
```

### try coalesce preserves Try on the fallback

> The ?? operator does not unwrap a Try fallback.

```ds
declare function getResult(): Result<int, Error>;
declare function getFallback(): Result<string, Error>;

const value = getResult() ?? getFallback();
value satisfies int | Result<string, Error>;
```

### try coalesce preserves Try fallback from nullish union

> The ?? operator does not unwrap a Try fallback from a nullish union.

```ds
declare function getMaybeResult(): Result<int, Error> | null;
declare function getFallback(): Result<string, Error>;

const value = getMaybeResult() ?? getFallback();
value satisfies int | Result<string, Error>;
```

### try coalesce unwraps one Try layer

> The ?? operator unwraps a single Try layer.

```ds
declare function getNested(): Result<Result<int, Error>, Error>;

const value = getNested() ?? 0;
value satisfies Result<int, Error> | int;
```

### try coalesce unwraps nested Try with nullish

> The ?? operator unwraps a single Try layer even with nullish unions.

```ds
declare function getNested(): Result<Result<int, Error>, Error> | null;

const value = getNested() ?? 0;
value satisfies Result<int, Error> | int;
```

### try coalesce removes nullish from unions

> The ?? operator removes nullish values before and after a Try unwrap.

```ds
declare function getMaybeResult(): Result<int, Error> | null | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce removes nullish from success type

> The ?? operator removes nullish from a Try success value.

```ds
declare function getMaybeResult(): Result<null, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce removes undefined from success type

> The ?? operator removes undefined from a Try success value.

```ds
declare function getMaybeResult(): Result<undefined, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

## Nullish coalesce

### nullish coalesce removes nullish types

> The ?? operator removes nullish values from unions.

```ds
declare function getMaybe(): string | null;

const value = getMaybe() ?? "fallback";
value satisfies string;
```

## Must unwrap

### must unwrap removes nullish types

> The ! operator removes nullish values from unions.

```ds
declare function getMaybe(): string | null;

const value = getMaybe()!;
value satisfies string;
```
