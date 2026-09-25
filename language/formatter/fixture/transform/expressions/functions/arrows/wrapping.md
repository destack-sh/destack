# Arrow Function Wrapping

## Line Breaking in Arrow Functions

### long arrow function breaks

Long arrow functions break at the assignment when needed.

```tspp line-width=40
const processItem = (item) => transformAndValidate(item)
```

```tspp expected
const processItem = (item) =>
    transformAndValidate(item);
```

### arrow function with long params breaks

Many parameters cause the param list to break.

```tspp line-width=40
const fn = (first, second, third, fourth) => first + second
```

```tspp expected
const fn = (
    first,
    second,
    third,
    fourth,
) => first + second;
```

### arrow function with complex return breaks

Complex return expressions break appropriately.

```tspp line-width=50
const handler = (event) => ({ type: event.type, target: event.target, timestamp: Date.now() })
```

```tspp expected
const handler = (event) => ({
    type: event.type,
    target: event.target,
    timestamp: Date.now(),
});
```

### arrow function with chained return breaks

Chain returns in arrow bodies keep each chain segment on its own line.

```tspp:main.tspp line-width=60
const normalize = (id) =>
  id
    .replace("@", resolve(__dirname, "./mods/"))
    .replace("#", resolve(__dirname, "../../"))
```

```tspp expected
const normalize = (id) =>
    id
        .replace("@", resolve(__dirname, "./mods/"))
        .replace("#", resolve(__dirname, "../../"));
```
