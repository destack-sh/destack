# Try and Coalesce Operators

Tests for the Try operator and nullish coalescing.

## Try unwrap

### _try unwrap uses Try branch

> The ? operator unwraps Try values to their success type.

```ds
declare function getResult(): Result<int, string>;

const value = getResult()?;
value satisfies int;
```

### _try unwrap preserves nullish success values

> The ? operator does not remove nullish from the success value.

```ds
declare function getResult(): Result<int | null, string>;

const value = getResult()?;
value satisfies int | null;
```

### _try unwrap unwraps only one Try layer

> The ? operator unwraps a single Try layer.

```ds
declare function getNested(): Result<Result<int, string>, string>;

const value = getNested()?;
value satisfies Result<int, string>;
```

### _try unwrap merges error types across unions

> The ? operator merges error types across Try unions.

```ds
declare function getResult(): Result<int, string> | Result<string, number>;

function read(): Result<int | string, string | number> {
    const value = getResult()?;
    value satisfies int | string;
    return Result.ok(value);
}
```

### _try unwrap rejects non Try values

> The ? operator requires a Try implementation.

```ds
const value = "nope"?;
value satisfies string;
```

- contains: no matching overload

### _try unwrap rejects nullish unions

> The ? operator does not accept nullish unions.

```ds
declare function getMaybeResult(): Result<int, string> | null;

const value = getMaybeResult()?;
value satisfies int;
```

- contains: no matching overload

### _try unwrap rejects unions with non Try values

> The ? operator rejects unions that contain non Try values.

```ds
declare function getResult(): Result<int, string> | string;

const value = getResult()?;
value satisfies int;
```

- contains: no matching overload

## Try coalesce

### _try coalesce uses Try branch

> The ?? operator uses Try when the left value implements it.

```ds
declare function getResult(): Result<int, string>;

const value = getResult() ?? 0;
value satisfies int;
```

### _try coalesce unwraps nullish and Try

> The ?? operator coalesces nullish before unwrapping Try.

```ds
declare function getMaybeResult(): Result<int, string> | null;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### _try coalesce removes nested nullish values

> The ?? operator removes nullish after unwrapping one Try layer.

```ds
declare function getMaybeResult(): Result<int | null, string>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### _try coalesce preserves fallback type when it differs

> The ?? operator returns a union of the unwrapped left and the fallback.

```ds
declare function getMaybeResult(): Result<int, string> | null;

const value = getMaybeResult() ?? "fallback";
value satisfies int | string;
```

### _try coalesce merges with non Try unions

> The ?? operator preserves non Try union members.

```ds
declare function getMaybeResult(): Result<int, string> | string;

const value = getMaybeResult() ?? 0;
value satisfies int | string;
```

### _try coalesce merges nullish and non Try unions

> The ?? operator removes nullish and keeps non Try members.

```ds
declare function getMaybeResult(): Result<int, string> | null | string;

const value = getMaybeResult() ?? 0;
value satisfies int | string;
```

### _try coalesce preserves Try on the fallback

> The ?? operator does not unwrap a Try fallback.

```ds
declare function getResult(): Result<int, string>;
declare function getFallback(): Result<string, number>;

const value = getResult() ?? getFallback();
value satisfies int | Result<string, number>;
```

### _try coalesce preserves Try fallback from nullish union

> The ?? operator does not unwrap a Try fallback from a nullish union.

```ds
declare function getMaybeResult(): Result<int, string> | null;
declare function getFallback(): Result<string, number>;

const value = getMaybeResult() ?? getFallback();
value satisfies int | Result<string, number>;
```

### _try coalesce unwraps one Try layer

> The ?? operator unwraps a single Try layer.

```ds
declare function getNested(): Result<Result<int, string>, string>;

const value = getNested() ?? 0;
value satisfies Result<int, string> | int;
```

### _try coalesce unwraps nested Try with nullish

> The ?? operator unwraps a single Try layer even with nullish unions.

```ds
declare function getNested(): Result<Result<int, string>, string> | null;

const value = getNested() ?? 0;
value satisfies Result<int, string> | int;
```

### _try coalesce removes nullish from unions

> The ?? operator removes nullish values before and after a Try unwrap.

```ds
declare function getMaybeResult(): Result<int, string> | null | undefined;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### _try coalesce removes nullish from success type

> The ?? operator removes nullish from a Try success value.

```ds
declare function getMaybeResult(): Result<null, string>;

const value = getMaybeResult() ?? 0;
value satisfies int;
```

### _try coalesce removes undefined from success type

> The ?? operator removes undefined from a Try success value.

```ds
declare function getMaybeResult(): Result<undefined, string>;

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
