# Catch

## error typing

### catch sees result errors

A propagated Result error becomes the catch value.

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

### catch sees unioned errors

Each Result alternative adds its error type to the catch value.

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

### catch sees multiple errors

Multiple propagated errors join in the catch value.

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

### catch can stop custom failures

A caught failure does not have to leave the function.

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

declare function getMaybe(): Maybe<int, string>;

const value = try {
    getMaybe()?;
    1
} catch (e) {
    e satisfies string;
    0
};
value satisfies int;
```
