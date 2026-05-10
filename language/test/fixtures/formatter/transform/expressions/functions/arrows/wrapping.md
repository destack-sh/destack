# Arrow Function Wrapping

## Line Breaking in Arrow Functions

### long arrow function breaks

Long arrow functions break at the assignment when needed.

```ds line-width=40
const processItem = (item) => transformAndValidate(item)
```

```ds expected
const processItem = (item) =>
    transformAndValidate(item);
```

### arrow function with long params breaks

Many parameters cause the param list to break.

```ds line-width=40
const fn = (first, second, third, fourth) => first + second
```

```ds expected
const fn = (
    first,
    second,
    third,
    fourth,
) => first + second;
```

### arrow function with complex return breaks

Complex return expressions break appropriately.

```ds line-width=50
const handler = (event) => ({ type: event.type, target: event.target, timestamp: Date.now() })
```

```ds expected
const handler = (event) => ({
    type: event.type,
    target: event.target,
    timestamp: Date.now(),
});
```

### arrow function with chained return breaks

Chain returns in arrow bodies keep each chain segment on its own line.

```ts:main.ts line-width=60
const normalize = (id) =>
  id
    .replace('@', resolve(__dirname, './mods/'))
    .replace('#', resolve(__dirname, '../../'))
```

```ts expected
const normalize = (id) =>
    id
        .replace("@", resolve(__dirname, "./mods/"))
        .replace("#", resolve(__dirname, "../../"));
```
