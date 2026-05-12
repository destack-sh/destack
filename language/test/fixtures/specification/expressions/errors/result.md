# Result Types

`Result<T, E>` is nominal, pattern-matchable, and has a small composition surface.

## constructors

### constructors return Result

Result constructors produce the declared result type.

```ds
const ok: Result<int, string> = Result.ok(1);
const err: Result<int, string> = Result.err("no");

ok satisfies Result<int, string>;
err satisfies Result<int, string>;
```

### variants require wrapping

Result is nominal and requires explicit construction.

```ds
const value: Result<int, string> = Ok { value: 1 };
```

- contains: not assignable

## narrowing

### patterns narrow result variants

Result patterns narrow to `Ok` and `Err` variants.

```ds
const value: Result<int, string> = Result.ok(1);

match (value) {
    Ok { value } => value satisfies int
    Err { error } => error satisfies string
}
```

## composition

### match folds variants

`Result.match` folds success and failure into one value.

```ds
const value: Result<int, string> = Result.ok(1);

const label = value.match({
    ok: (number) => `ok:${number}`,
    err: (error) => `err:${error}`,
});

label satisfies string;
```

### tap keeps the original Result

`tap` and `tapErr` observe one branch without changing the result type.

```ds
declare function logValue(value: int): void;
declare function logError(error: string): void;

const value: Result<int, string> = Result.ok(1);
const out = value.tap(logValue).tapErr(logError);

out satisfies Result<int, string>;
```

## async

### AsyncResult wraps Promise Result

`AsyncResult<T, E>` keeps promise rejection outside typed error flow.

```ds
newtype NetworkError = string;

declare function request(): Promise<string>;
declare function recover(error: unknown): NetworkError;

const value = AsyncResult.fromPromise(request(), recover);

value satisfies AsyncResult<string, NetworkError>;
```

### async failures can be propagated

`AsyncResult` can rebuild itself from propagated failures.

```ds
newtype NetworkError = string;
newtype DecodeError = string;
struct User {
    name: string;
}

declare function request(): AsyncResult<string, NetworkError>;
declare function decode(raw: string): Result<User, DecodeError>;

async function load(): AsyncResult<User, NetworkError | DecodeError> {
    const raw = (await request())?;
    const user = decode(raw)?;
    return Result.ok(user);
}
```
