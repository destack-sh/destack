# Async Arrow Functions

## Async Arrow Functions

### async arrow function

The `async` keyword precedes the parameter list.

```tspp
const f = async (x) => x
```

```tspp expected
const f = async (x) => x;
```

### async arrow function with await

Async functions can use `await` in their body.

```tspp
const f = async (url) => await fetch(url)
```

```tspp expected
const f = async (url) => await fetch(url);
```

### async arrow function with block

Async arrow functions with blocks expand normally.

```tspp
const f = async (url) => { const res = await fetch(url); return res.json() }
```

```tspp expected
const f = async (url) => {
    const res = await fetch(url);
    return res.json();
};
```
