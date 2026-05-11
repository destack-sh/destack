# TryFrom And TryInto

## fallible

Fallible conversion uses `TryFrom`.

### TryFrom returns Result

```ds
struct ParseIdError {
    message: string;
}

newtype UserId = string;

extension of UserId implements TryFrom<string, ParseIdError> {
    static tryFrom(value: string): Result<UserId, ParseIdError> {
        return Result.ok(UserId(value));
    }
}

const id = UserId.tryFrom("u_123")?;
id satisfies UserId;
```

### TryInto is provided by TryFrom

```ds
struct ParseIdError {
    message: string;
}

newtype UserId = string;

extension of UserId implements TryFrom<string, ParseIdError> {
    static tryFrom(value: string): Result<UserId, ParseIdError> {
        return Result.ok(UserId(value));
    }
}

const source = "u_123";
const id: UserId = source.tryInto()?;
id satisfies UserId;
```
