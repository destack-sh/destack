# Promises

Tests for Promise<T> and await expressions.

## Await

### await unwraps promise value type

```test libs=es2015.promise
```

```ds
declare const value: Promise<number>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
```

### await keeps non promise values

```test libs=es2015.promise
```

```ds
async function read(): Promise<string> {
    const value = await "hello";
    value satisfies string;
    return value;
}
```
