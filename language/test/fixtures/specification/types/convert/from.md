# From And Into

## infallible

Infallible conversion uses `From`.

### From converts into the implementing type

```ds
newtype UserId = string;

extension of UserId implements From<string> {
    static from(value: string): UserId {
        return UserId(value);
    }
}

const id = UserId.from("u_123");
id satisfies UserId;
```

### Into is provided by From

```ds
newtype UserId = string;

extension of UserId implements From<string> {
    static from(value: string): UserId {
        return UserId(value);
    }
}

const source = "u_123";
const id: UserId = source.into();
id satisfies UserId;
```

## chaining

Conversions are explicit one-step operations.

### Into does not chain through intermediate conversions

```ds error
newtype UserId = string;
newtype AccountId = UserId;

extension of UserId implements From<string> {
    static from(value: string): UserId {
        return UserId(value);
    }
}

extension of AccountId implements From<UserId> {
    static from(value: UserId): AccountId {
        return AccountId(value);
    }
}

const source = "u_123";
const id: AccountId = source.into();
```

```error
- contains: no conversion from string to AccountId
```
