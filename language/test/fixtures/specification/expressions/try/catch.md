# Try Catch Variables

## useUnknownInCatchVariables

### useUnknownInCatchVariables true uses unknown

> Catch variables default to unknown when enabled.

```json:destack.json
{ "compiler": { "useUnknownInCatchVariables": true } }
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
type BrokenBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct BrokenTry<T, E> {
    value: BrokenBranch<T, E>;
}

extension<T, E> of BrokenTry<T, E> implements Try<T, E> {
    branch(): BrokenBranch<T, E> {
        this.value
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
type BrokenBranch<T, E> =
    | { kind: "ok", value: T }
    | { kind: "err", error: E };

struct BrokenTry<T, E> {
    value: BrokenBranch<T, E>;
}

extension<T, E> of BrokenTry<T, E> implements Try<T, E> {
    branch(): BrokenBranch<T, E> {
        this.value
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

```json:destack.json
{ "compiler": { "noAny": false, "useUnknownInCatchVariables": false } }
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

## Typed catch annotations

### typescript catch annotation allows any

> TypeScript accepts `any` in catch annotations and applies it to the binding.

```json:destack.json
{ "compiler": { "noAny": false } }
```

```ts:main.ts
const value = try {
    throw { message: "oops" }
} catch ({ message }: any) {
    message satisfies string;
    0
};
value satisfies number;
```

### typescript catch annotation allows unknown

> TypeScript accepts `unknown` in catch annotations.

```ts:main.ts
try {
    throw 1;
} catch (err: unknown) {
    err satisfies string;
}
```

- contains: expected string

### typescript catch annotation rejects concrete types

> TypeScript restricts catch annotations to `any` and `unknown`.

```ts:main.ts
try {
    throw 1
} catch (err: string) {
    0
}
```

- catch type annotations must be 'any' or 'unknown'