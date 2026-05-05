# Coalescing

The `??` operator keeps the value or evaluates the fallback.

## results

### Try failures use fallback

> A failed Try value uses the fallback.

```ds
declare function getResult(): Result<int, Error>;

const value = getResult() ?? 0;
value satisfies int;
```

### outer nullish values use fallback

> Null and undefined on the left also use the fallback.

```ds
declare function getMaybeResult(): Result<int, Error> | null | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### nullish success values use fallback

> A null success value also uses the fallback.

```ds
declare function getMaybeResult(): Result<int | null, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### nullish failures use fallback

> Nullish values inside the opened layer use the fallback.

```ds
declare function getMaybeResult(): Result<int | null | undefined, Error | null> | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### fallback types join

> A fallback with a different type joins the result.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

const value = getMaybeResult() ?? "fallback";
value satisfies int | string;
```

### fallback Results are not unwrapped

> The fallback expression is not unwrapped.

```ds
declare function getResult(): Result<int, Error>;
declare function getFallback(): Result<string, Error>;

const value = getResult() ?? getFallback();
value satisfies int | Result<string, Error>;
```

### coalescing opens one Try layer

> Coalescing opens only one Try layer.

```ds
declare function getNested(): Result<Result<int, Error>, Error>;

const value = getNested() ?? 0;
value satisfies Result<int, Error> | int;
```

## custom

### custom Try failures use fallback

Coalescing uses the fallback instead of propagating failure.

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

### Try and non-Try unions need narrowing

> A union of Try and unrelated values must be narrowed first.

```ds
declare function getMaybeResult(): Result<int, Error> | string;

const value = getMaybeResult() ?? 0;
```

- contains: no matching overload

### nullish mixed unions need narrowing

> Nullish members are allowed, but unrelated members must be narrowed first.

```ds
declare function getMaybeResult(): Result<int, Error> | null | string;

const value = getMaybeResult() ?? 0;
```

- contains: no matching overload
