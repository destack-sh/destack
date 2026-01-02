# Promises

Tests for Promise<T> and await expressions.

## Await

### await unwraps promise value type


```ds libs=es2015.promise
declare const value: Promise<number>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await rejects non promise values


```ds libs=es2015.promise
async function read(): Promise<string> {
    const value = await "hello";
    value satisfies string;
    return value;
}
```

- contains: not assignable

### await distributes over unions


```ds libs=es2015.promise
declare const value: Promise<number> | Promise<string>;

async function read(): Promise<number | string> {
    const inner = await value;
    inner satisfies number | string;
    return inner;
}
```

### await unwraps nested promises


```ds libs=es2015.promise
declare const value: Promise<Promise<number>>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await preserves any values


```ds libs=es2015.promise
declare const value: any;

async function read(): Promise<any> {
    const inner = await value;
    inner satisfies any;
    return inner;
}
```

### await preserves unknown promise values


```ds libs=es2015.promise
declare const value: Promise<unknown>;

async function read(): Promise<unknown> {
    const inner = await value;
    inner satisfies unknown;
    return inner;
}
```

### _await unwraps promise aliases

> TODO #Incomplete: await should unwrap generic aliases to Promise.


```ds libs=es2015.promise
type Box<T> = Promise<T>;
declare const value: Box<int32>;

async function read(): Promise<int32> {
    const inner = await value;
    inner satisfies int32;
    return inner;
}
```
