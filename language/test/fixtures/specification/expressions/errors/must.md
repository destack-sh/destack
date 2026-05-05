# Must

Postfix `!` keeps the value or traps.

## values

### postfix ! removes nullish values

Nullish values trap and are removed from the result type.

```ds
declare const value: string | null | undefined;

const out = value!;
out satisfies string;
```

### postfix ! traps on Result failures

Try failures and nullish success values trap.

```ds
declare const value: Result<string | null | undefined, Error | null> | undefined;

const out = value!;
out satisfies string;
```

### postfix ! opens one Try layer

Nested Try values stay wrapped.

```ds
declare const value: Result<Result<string, Error>, Error>;

const out = value!;
out satisfies Result<string, Error>;
```

## narrowing

### mixed Try unions need narrowing

A union of Try and unrelated values must be narrowed first.

```ds
declare const value: Result<string, Error> | number | null;

const out = value!;
```

- contains: narrow
