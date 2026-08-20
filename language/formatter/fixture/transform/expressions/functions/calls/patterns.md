# Call Patterns

Call pattern fixtures cover callbacks, nested calls, curried calls, and awaited call chains.

## Chained Operations with Callbacks

### chain with multiple callbacks

Method chains with callback arguments at each step.

```ds line-width=50
data.filter((x) => x.active).map((x) => x.name).reduce((a, b) => a + b)
```

```ds expected
data.filter((x) => x.active)
    .map((x) => x.name)
    .reduce((a, b) => a + b);
```

### chain with block callback

When a callback has a block body, the chain continues after the closing brace.

```ds line-width=40
items.map((item) => { return item.value }).filter((v) => v > 0)
```

```ds expected
items
    .map((item) => {
        return item.value;
    })
    .filter((v) => v > 0);
```

### nested chains in callback

Chains inside callback bodies break when they exceed line width.

```ds line-width=50
outer.map((x) => x.items.filter((y) => y.ok).map((y) => y.value))
```

```ds expected
outer.map((x) =>
    x.items
        .filter((y) => y.ok)
        .map((y) => y.value),
);
```

## Argument Shapes

### call with mixed argument types

Function calls with various argument types.

```ds line-width=50
createEntity("user", 42, { role: "admin" }, ["read", "write"], (err) => handle(err))
```

```ds expected
createEntity(
    "user",
    42,
    { role: "admin" },
    ["read", "write"],
    (err) => handle(err),
);
```

### call with long generic types

Generic type arguments can get long.

```ds line-width=50
fetchData<Response<User>, ErrorType, Options>(url, config)
```

```ds expected
fetchData<Response<User>, ErrorType, Options>(
    url,
    config,
);
```

### nested function calls

Deeply nested function calls.

```ds
outer(middle(inner(value)))
```

```ds expected
outer(middle(inner(value)));
```

### nested calls that break

When nested calls don't fit, they break appropriately.

```ds line-width=30
outer(middle(inner(longValue)))
```

```ds expected
outer(
    middle(inner(longValue)),
);
```

## Deeply Nested Callbacks

### nested callbacks in chain

Deeply nested callbacks within method chains.

```ds line-width=50
fetch(url).then((res) => res.json()).then((data) => process(data)).catch((err) => handle(err))
```

```ds expected
fetch(url)
    .then((res) => res.json())
    .then((data) => process(data))
    .catch((err) => handle(err));
```

### callback inside callback

Callbacks passed as arguments to other callbacks.

```ds line-width=50
outer((x) => inner((y) => transform(x, y)))
```

```ds expected
outer((x) => inner((y) => transform(x, y)));
```

### deeply nested callback breaks

Very deep nesting breaks appropriately.

```ds line-width=40
a((x) => b((y) => c((z) => d(x, y, z))))
```

```ds expected
a((x) =>
    b((y) => c((z) => d(x, y, z))),
);
```

## Curried Function Calls

### curried call

Curried function application.

```ds
curry(a)(b)(c)
```

```ds expected
curry(a)(b)(c);
```

### long curried call breaks inner tail first

Long curried calls break the inner tail before the final call when width runs out.

```ds line-width=30
curriedFunction(firstArg)(secondArg)(thirdArg)
```

```ds expected
curriedFunction(firstArg)(
    secondArg,
)(thirdArg);
```

### curried call with objects

Curried calls keep simple tails together and break object tails.

```ds line-width=40
configure({ mode: "dev" })({ debug: true })({ verbose: false })
```

```ds expected
configure({ mode: "dev" })({
    debug: true,
})({ verbose: false });
```

## Async/Await Patterns

### await call

Await expressions format normally.

```ds
const data = await fetch(url)
```

```ds expected
const data = await fetch(url);
```

### await in chain

Await works with method chains.
Long expressions use method chaining style.

```ds line-width=40
const json = await fetch(url).then((r) => r.json())
```

```ds expected
const json = await fetch(url).then(
    (r) => r.json(),
);
```

### multiple awaits in expression

Multiple awaits in one expression.

```ds
const result = await process(await fetch(url))
```

```ds expected
const result = await process(await fetch(url));
```

### async arrow function

Async arrow functions.

```ds
const handler = async (event) => await processEvent(event)
```

```ds expected
const handler = async (event) => await processEvent(event);
```

### async iife

Async immediately invoked function expression.

```ds
(async () => { const data = await fetch(url); return data })()
```

```ds expected
(async () => {
    const data = await fetch(url);
    return data;
})();
```
