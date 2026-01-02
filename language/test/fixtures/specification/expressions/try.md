# Try Expressions

Tests for try/catch/finally expression behavior.

## Try expression

### try expression without catch is invalid

> A try expression requires a catch or finally block.

```ds
const value = try { 1 };
value satisfies int;
```

- contains: requires a catch or finally

### try expression returns body type with finally

> Finally does not affect the try expression type.

```ds
const value = try {
    1
} finally {
    2
};
value satisfies int;
```

### _try expression does not unwrap Try values

> Try does not unwrap Try values without ?.

```ds
declare function getResult(): Result<int, string>;

const value = try {
    getResult()
} catch e {
    e satisfies Error;
    getResult()
};
value satisfies Result<int, string>;
```

### _try expression does not implicitly unwrap

> Implicit Try unwrap is not allowed.

```ds
declare function getResult(): Result<int, string>;

const value = try {
    getResult()
} catch e {
    e satisfies Error;
    0
};
value satisfies int;
```

- contains: not assignable

## Try and catch

### _try catch returns union type

> Catch contributes to the try expression type.

```ds
const value = try {
    1
} catch e {
    e satisfies Error;
    "fallback"
};
value satisfies int | string;
```

### _try catch handles explicit Try propagation

> ? propagates into catch.

```ds
declare function getResult(): Result<int, string>;

const value = try {
    getResult()?;
} catch e {
    e satisfies Error | string;
    "fallback"
};
value satisfies int | string;
```

### _try catch does not unwrap without ?

> Without ?, Try values remain unwrapped.

```ds
declare function getResult(): Result<int, string>;

const value = try {
    getResult()
} catch e {
    e satisfies Error;
    0
};
value satisfies Result<int, string> | int;
```

## Try and finally

### try finally keeps body type

> Finally does not affect the try expression type.

```ds
const value = try {
    1
} finally {
    2
};
value satisfies int;
```

## Try catch finally

### _try catch finally returns union type

> Catch contributes to the try expression type.

```ds
const value = try {
    1
} catch e {
    e satisfies Error;
    "fallback"
} finally {
    2
};
value satisfies int | string;
```
