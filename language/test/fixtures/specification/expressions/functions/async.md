# Promises

Promise<T> and await expressions.

## await

### await unwraps promise value type

Await unwraps the resolved value type of a promise.

```ds
declare const value: Promise<number>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await rejects non promise values

Await only accepts typed async values.

```ds
async function read(): Promise<string> {
    const value = await "hello";
    value satisfies string;
    return value;
}
```

- contains: not assignable

### await distributes over unions

Await distributes across promise unions.

```ds
declare const value: Promise<number> | Promise<string>;

async function read(): Promise<number | string> {
    const inner = await value;
    inner satisfies number | string;
    return inner;
}
```

### await unwraps nested promises

Await recursively unwraps nested promise values.

```ds
declare const value: Promise<Promise<number>>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await preserves unknown promise values

Await preserves `unknown` when the promise value is unknown.

```ds
declare const value: Promise<unknown>;

async function read(): Promise<unknown> {
    const inner = await value;
    inner satisfies unknown;
    return inner;
}
```

### await unwraps promise aliases

Await unwraps promise aliases through ordinary alias resolution.

```ds
type Box<T> = Promise<T>;
declare const value: Box<int32>;

async function read(): Promise<int32> {
    const inner = await value;
    inner satisfies int32;
    return inner;
}
```

### await unwraps unevaluated promise arguments

Await unwraps Promise arguments even when the static argument is an unevaluated alias.

```ds
type Box<T> = Promise<T>;
type Alias = Box<string>;
declare const value: Alias;

async function read(): Promise<string> {
    const inner = await value;
    inner satisfies string;
    return inner;
}
```

### await! unwraps async result values

`await!` awaits a fallible promise and unwraps the success value.

```ds
declare const value: Promise<Result<string, Error>>;

async function read(): Promise<string> {
    const inner = await! value;
    inner satisfies string;
    return inner;
}
```
