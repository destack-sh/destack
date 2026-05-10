# Async Functions

## Async Functions

### async function

The `async` keyword precedes `function` with a single space between them.

```ds
async   function   foo  (  )   {   }
```

```ds expected
async function foo() {}
```

### async function with await

Await expressions are preserved inside async function bodies.

```ds
async function fetch(url: string) { const res = await request(url); return res }
```

```ds expected
async function fetch(url: string) {
    const res = await request(url);
    return res;
}
```

### async function with return type

Async functions typically return Promise types.

```ds
async function getData(): Promise<Data> { return await fetchData() }
```

```ds expected
async function getData(): Promise<Data> {
    return await fetchData();
}
```
