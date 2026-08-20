# Async Arrow Functions

## Async Arrow Functions

### async arrow function

The `async` keyword precedes the parameter list.

```ds
const f = async (x) => x
```

```ds expected
const f = async (x) => x;
```

### async arrow function with await

Async functions can use `await` in their body.

```ds
const f = async (url) => await fetch(url)
```

```ds expected
const f = async (url) => await fetch(url);
```

### async arrow function with block

Async arrow functions with blocks expand normally.

```ds
const f = async (url) => { const res = await fetch(url); return res.json() }
```

```ds expected
const f = async (url) => {
    const res = await fetch(url);
    return res.json();
};
```
