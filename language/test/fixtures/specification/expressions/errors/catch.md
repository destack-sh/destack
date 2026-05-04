# Try Catch Variables

## error typing

### try catch uses Try error type

> Catch variables use the Try error type from ? expressions.

```ds
declare function readConfig(): Result<int, string>;

const value = try {
    readConfig()?;
    1
} catch (e) {
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
} catch (e) {
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
} catch (e) {
    e satisfies "missing" | "bad";
    0
};
value satisfies int;
```

### try catch handles custom Try failures

> Catching a Try error does not require FromFailure.

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

declare function getBroken(): BrokenTry<int, string>;

const value = try {
    getBroken()?;
    1
} catch (e) {
    e satisfies string;
    0
};
value satisfies int;
```

### try catch handles custom Try failures in functions

> Catching a Try error allows non Try return types.

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

declare function getBroken(): BrokenTry<int, string>;

function read(): int {
    const value = try {
        getBroken()?;
        1
    } catch (e) {
        e satisfies string;
        0
    };
    return value;
}
```

## typed catch annotations

### catch annotation allows unknown

> Accepts `unknown` in catch annotations.

```ts:main.ts
try {
    throw 1;
} catch (err: unknown) {
    err satisfies string;
}
```

- contains: not assignable

### catch annotation rejects concrete types

> Catch annotations only allow `unknown`.

```ts:main.ts
try {
    throw 1
} catch (err: string) {
    0
}
```

- contains: catch type annotations must be unknown
