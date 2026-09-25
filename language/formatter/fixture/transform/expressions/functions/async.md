# Async Functions

## Async Functions

### async function

The `async` keyword precedes `function` with a single space between them.

```tspp
async   function   foo  (  )   {   }
```

```tspp expected
async function foo() {}
```

### async function with await

Await expressions are preserved inside async function bodies.

```tspp
async function fetch(url: string) { const res = await request(url); return res }
```

```tspp expected
async function fetch(url: string) {
    const res = await request(url);
    return res;
}
```

### async function with return type

Async functions typically return Promise types.

```tspp
async function getData(): Promise<Data> { return await fetchData() }
```

```tspp expected
async function getData(): Promise<Data> {
    return await fetchData();
}
```
