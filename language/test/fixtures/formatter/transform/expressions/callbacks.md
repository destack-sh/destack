# Callback Arguments

Tests for callback-last argument formatting (Prettier-style).

## Callback Hugging

### callback last argument hugs

When the last argument is a callback, it should expand while keeping other args on the first line.

```ds line-width=40
fetchData(url, options, (response) => { process(response) })
```

```ds expected
fetchData(url, options, (response) => {
    process(response)
});
```

### single callback argument

A single callback argument hugs the parentheses.

```ds
array.forEach((item) => { console.log(item) })
```

```ds expected
array.forEach((item) => {
    console.log(item)
});
```

### callback with multiple statements

Multi-statement callbacks always expand.

```ds
items.map((item) => { const x = item.value; return x * 2 })
```

```ds expected
items.map((item) => {
    const x = item.value;
    return x * 2;
});
```

## Object/Array Arguments

### object argument expands

Object arguments expand when they exceed line width, hugging the parentheses.

```ds line-width=30
configure({ debug: true, verbose: false })
```

```ds expected
configure({
    debug: true,
    verbose: false,
});
```

### short array stays compact

Short arrays stay on one line when they fit.

```ds line-width=40
process([1, 2, 3, 4, 5, 6])
```

```ds expected
process([1, 2, 3, 4, 5, 6]);
```

### multiple args break when long

Long argument lists break to multiple lines.

```ds line-width=40
createUser("john", "doe", { role: "admin", active: true })
```

```ds expected
createUser("john", "doe", {
    role: "admin",
    active: true,
});
```
