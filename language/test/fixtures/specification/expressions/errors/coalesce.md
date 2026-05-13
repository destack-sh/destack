# Coalescing

The `??` operator keeps the value or evaluates the fallback.
It falls back on outer nullish values, Try failures, and nullish values inside the opened success value.
It does not open nested Try values or distribute over unrelated union arms.

## results

### fallback preserves opened success type

Numeric fallbacks keep the opened success type when the literal fits.

```ds
declare function getResult(): Result<int, Error>;

const value = getResult() ?? 0;
value satisfies int;
```

### nullish wrappers preserve opened success type

Nullish wrappers use the fallback without widening the opened success type.

```ds
declare function getMaybeResult(): Result<int, Error> | null | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### nullish success values preserve opened success type

Nullish success values use the fallback without widening the opened success type.

```ds
declare function getMaybeResult(): Result<int | null, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### nullish failures preserve opened success type

Nullish values inside the opened layer use the fallback without widening the opened success type.

```ds
declare function getMaybeResult(): Result<int | null | undefined, Error | null> | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### fallback types join

A fallback with a different type joins the opened success type.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

const value = getMaybeResult() ?? "fallback";
value satisfies int | string;
```

### fallback Results are not unwrapped

The fallback expression is ordinary code, even when it is another Result.

```ds
declare function getResult(): Result<int, Error>;
declare function getFallback(): Result<string, Error>;

const value = getResult() ?? getFallback();
value satisfies int | Result<string, Error>;
```

### coalescing opens one Try layer

Only the outer Try layer is opened.

```ds
declare function getNested(): Result<Result<int, Error>, Error>;

const value = getNested() ?? 0;
value satisfies Result<int, Error> | int;
```

### Try carrier unions join success types

Coalescing can open a union when every non-nullish arm is a Try carrier.

```ds
struct MissingError {
    message: string;
}

extension of MissingError implements Error {
    display(): string {
        this.message
    }
}

struct FormatError {
    message: string;
}

extension of FormatError implements Error {
    display(): string {
        this.message
    }
}

declare function getResult(): Result<int, MissingError> | Result<string, FormatError>;

const value = getResult() ?? false;
value satisfies int | string | boolean;
```

## custom

### custom Try fallback preserves opened success type

Custom Try implementors use the same fallback typing.

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

declare function getMaybe(): Maybe<int, Error>;

const value = getMaybe() ?? 0;
value satisfies int;
```

## narrowing

### mixed Try unions need narrowing

A union of Try and unrelated values must be narrowed first.

```ds
declare function getMaybeResult(): Result<int, Error> | string;

const value = getMaybeResult() ?? 0;
```

- contains: narrow

### nullish mixed unions need narrowing

Nullish members are allowed, but unrelated members must be narrowed first.

```ds
declare function getMaybeResult(): Result<int, Error> | null | string;

const value = getMaybeResult() ?? 0;
```

- contains: narrow
