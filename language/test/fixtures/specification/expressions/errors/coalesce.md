# Coalescing

Nullish coalescing removes nullish values and can unwrap `Try` values on the left side.

## try coalesce

### try coalesce uses Try branch

> The `??` operator uses Try when the left value implements it.

```ds
declare function getResult(): Result<int, Error>;

const value = getResult() ?? 0;
value satisfies int;
```

### try coalesce handles custom Try failures

> The `??` operator does not require FromFailure.

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

const value = getBroken() ?? 0;
value satisfies int;
```

### try coalesce unwraps nullish and Try

> The `??` operator coalesces nullish before unwrapping Try.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce removes nested nullish values

> The `??` operator removes nullish after unwrapping one Try layer.

```ds
declare function getMaybeResult(): Result<int | null, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce preserves fallback type when it differs

> The `??` operator returns a union of the unwrapped left and the fallback.

```ds
declare function getMaybeResult(): Result<int, Error> | null;

const value = getMaybeResult() ?? "fallback";
value satisfies int | string;
```

### try coalesce rejects non Try unions

> The `??` operator requires arbitrary Try and non-Try unions to be narrowed first.

```ds
declare function getMaybeResult(): Result<int, Error> | string;

const value = getMaybeResult() ?? 0;
```

- contains: no matching overload

### try coalesce rejects nullish and non Try unions

> The `??` operator allows nullish Try unions but not arbitrary non-Try union members.

```ds
declare function getMaybeResult(): Result<int, Error> | null | string;

const value = getMaybeResult() ?? 0;
```

- contains: no matching overload

### try coalesce preserves Try on the fallback

> The `??` operator does not unwrap a Try fallback.

```ds
declare function getResult(): Result<int, Error>;
declare function getFallback(): Result<string, Error>;

const value = getResult() ?? getFallback();
value satisfies int | Result<string, Error>;
```

### try coalesce preserves Try fallback from nullish union

> The `??` operator does not unwrap a Try fallback from a nullish union.

```ds
declare function getMaybeResult(): Result<int, Error> | null;
declare function getFallback(): Result<string, Error>;

const value = getMaybeResult() ?? getFallback();
value satisfies int | Result<string, Error>;
```

### try coalesce unwraps one Try layer

> The `??` operator unwraps a single Try layer.

```ds
declare function getNested(): Result<Result<int, Error>, Error>;

const value = getNested() ?? 0;
value satisfies Result<int, Error> | int;
```

### try coalesce unwraps nested Try with nullish

> The `??` operator unwraps a single Try layer even with nullish unions.

```ds
declare function getNested(): Result<Result<int, Error>, Error> | null;

const value = getNested() ?? 0;
value satisfies Result<int, Error> | int;
```

### try coalesce removes nullish from unions

> The `??` operator removes nullish values before and after a Try unwrap.

```ds
declare function getMaybeResult(): Result<int, Error> | null | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce removes nullish from success type

> The `??` operator removes nullish from a Try success value.

```ds
declare function getMaybeResult(): Result<null, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### try coalesce removes undefined from success type

> The `??` operator removes undefined from a Try success value.

```ds
declare function getMaybeResult(): Result<undefined, Error>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```
