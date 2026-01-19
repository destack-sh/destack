# Try Catch Variables

## useUnknownInCatchVariables

### useUnknownInCatchVariables true uses unknown

> Catch variables default to unknown when enabled.

```ds:dsconfig.json
{ "compilerOptions": { "useUnknownInCatchVariables": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    1
} catch e {
    e satisfies string;
    0
};
value satisfies int;
```

- contains: expected string

## Try error typing

### try catch uses Try error type

> Catch variables use the Try error type from ? expressions.

```ds
declare function readConfig(): Result<int, string>;

const value = try {
    readConfig()?;
    1
} catch e {
    e satisfies string;
    2
};
value satisfies int;
```

### try catch unions Try error types

> Catch variables union error types across Try unions.

```ds
declare function readConfig(): Result<int, "missing"> | Result<int, "bad">;

const value = try {
    readConfig()?;
    1
} catch e {
    e satisfies "missing" | "bad";
    2
};
value satisfies int;
```

### try catch unions multiple Try errors

> Catch variables union error types across multiple ? expressions.

```ds
declare function readConfig(): Result<int, "missing">;
declare function readVersion(): Result<int, "bad">;

const value = try {
    readConfig()?;
    readVersion()?;
    1
} catch e {
    e satisfies "missing" | "bad";
    0
};
value satisfies int;
```

### try catch allows missing fromError

> Catching a Try error does not require Try.fromError.

```ds
struct BrokenOk<T> {
    kind: "ok" = "ok",
    value: T,
}

struct BrokenErr<E> {
    kind: "err" = "err",
    error: E,
}

newtype BrokenTry<T, E> = BrokenOk<T> | BrokenErr<E>;

extension<T, E> for BrokenTry<T, E> implements Try<T, E> {
    branch(): BrokenTry<T, E> {
        this
    }
}

declare function getBroken(): BrokenTry<int, string>;

const value = try {
    getBroken()?;
    1
} catch e {
    e satisfies string;
    0
};
value satisfies int;
```

### try catch allows missing fromError in functions

> Catching a Try error allows non Try return types.

```ds
struct BrokenOk<T> {
    kind: "ok" = "ok",
    value: T,
}

struct BrokenErr<E> {
    kind: "err" = "err",
    error: E,
}

newtype BrokenTry<T, E> = BrokenOk<T> | BrokenErr<E>;

extension<T, E> for BrokenTry<T, E> implements Try<T, E> {
    branch(): BrokenTry<T, E> {
        this
    }
}

declare function getBroken(): BrokenTry<int, string>;

function read(): int {
    const value = try {
        getBroken()?;
        1
    } catch e {
        e satisfies string;
        0
    };
    return value;
}
```

### useUnknownInCatchVariables false uses any

> Catch variables default to any when disabled.

```ds:dsconfig.json
{ "compilerOptions": { "noAny": false, "useUnknownInCatchVariables": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value = try {
    1
} catch e {
    e satisfies string;
    0
};
value satisfies int;
```
