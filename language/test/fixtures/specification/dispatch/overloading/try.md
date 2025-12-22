# Try and Coalesce Operators

Tests for the Try operator and nullish coalescing.

## Try unwrap

> TODO #Broken: fix try operator resolution
 (this should mostly work? but we just get the Result type..)

### _try unwrap uses Try branch

> The ? operator unwraps Try values to their success type.

```ds
declare function getResult(): Result<int, string>;

const value = getResult()?;
value satisfies int;
```

## Try coalesce

### _try coalesce uses Try branch

> The ?? operator uses Try when the left value implements it.

```ds
declare function getResult(): Result<int, string>;

const value = getResult() ?? 0;
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
