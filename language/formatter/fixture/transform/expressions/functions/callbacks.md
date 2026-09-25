# Callback Arguments

Callback fixtures cover callback-last arguments.

## Callback Hugging

### callback last argument hugs

When the last argument is a callback, it should expand while keeping other args on the first line.

```tspp line-width=40
fetchData(url, options, (response) => { process(response) })
```

```tspp expected
fetchData(url, options, (response) => {
    process(response)
});
```

### single callback argument

A single callback argument hugs the parentheses.

```tspp
array.forEach((item) => { console.log(item) })
```

```tspp expected
array.forEach((item) => {
    console.log(item)
});
```

### callback with multiple statements

Multi-statement callbacks always expand.

```tspp
items.map((item) => { const x = item.value; return x * 2 })
```

```tspp expected
items.map((item) => {
    const x = item.value;
    return x * 2;
});
```

## Object/Array Arguments

### object argument expands

Object arguments expand when they exceed line width, hugging the parentheses.

```tspp line-width=30
configure({ debug: true, verbose: false })
```

```tspp expected
configure({
    debug: true,
    verbose: false,
});
```

### short array stays compact

Short arrays stay on one line when they fit.

```tspp line-width=40
process([1, 2, 3, 4, 5, 6])
```

```tspp expected
process([1, 2, 3, 4, 5, 6]);
```

### multiple args break when long

Long argument lists break to multiple lines.

```tspp line-width=40
createUser("john", "doe", { role: "admin", active: true })
```

```tspp expected
createUser("john", "doe", {
    role: "admin",
    active: true,
});
```
