# Promises

Promise<T> and await expressions.

## Await

### await unwraps promise value type


```ds libs=es5,es2015.promise
declare const value: Promise<number>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await rejects non promise values


```ds libs=es5,es2015.promise
async function read(): Promise<string> {
    const value = await "hello";
    value satisfies string;
    return value;
}
```

- contains: not assignable

### await distributes over unions


```ds libs=es5,es2015.promise
declare const value: Promise<number> | Promise<string>;

async function read(): Promise<number | string> {
    const inner = await value;
    inner satisfies number | string;
    return inner;
}
```

### await unwraps nested promises


```ds libs=es5,es2015.promise
declare const value: Promise<Promise<number>>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await preserves unknown promise values


```ds libs=es5,es2015.promise
declare const value: Promise<unknown>;

async function read(): Promise<unknown> {
    const inner = await value;
    inner satisfies unknown;
    return inner;
}
```

### await unwraps promise aliases

```ds libs=es5,es2015.promise
type Box<T> = Promise<T>;
declare const value: Box<int32>;

async function read(): Promise<int32> {
    const inner = await value;
    inner satisfies int32;
    return inner;
}
```

### await unwraps unevaluated promise arguments

> Await unwraps Promise arguments even when the static argument is an unevaluated alias.

```ds libs=es5,es2015.promise
type Box<T> = Promise<T>;
type Alias = Box<string>;
declare const value: Alias;

async function read(): Promise<string> {
    const inner = await value;
    inner satisfies string;
    return inner;
}
```
