# Call Patterns

Call pattern fixtures cover callbacks, nested calls, curried calls, and awaited call chains.

## Chained Operations with Callbacks

### chain with multiple callbacks

Method chains with callback arguments at each step.

```tspp line-width=50
data.filter((x) => x.active).map((x) => x.name).reduce((a, b) => a + b)
```

```tspp expected
data.filter((x) => x.active)
    .map((x) => x.name)
    .reduce((a, b) => a + b);
```

### chain with block callback

When a callback has a block body, the chain continues after the closing brace.

```tspp line-width=40
items.map((item) => { return item.value }).filter((v) => v > 0)
```

```tspp expected
items
    .map((item) => {
        return item.value;
    })
    .filter((v) => v > 0);
```

### nested chains in callback

Chains inside callback bodies break when they exceed line width.

```tspp line-width=50
outer.map((x) => x.items.filter((y) => y.ok).map((y) => y.value))
```

```tspp expected
outer.map((x) =>
    x.items
        .filter((y) => y.ok)
        .map((y) => y.value),
);
```

## Argument Shapes

### call with mixed argument types

Function calls with various argument types.

```tspp line-width=50
createEntity("user", 42, { role: "admin" }, ["read", "write"], (err) => handle(err))
```

```tspp expected
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

```tspp line-width=50
fetchData<Response<User>, ErrorType, Options>(url, config)
```

```tspp expected
fetchData<Response<User>, ErrorType, Options>(
    url,
    config,
);
```

### nested function calls

Deeply nested function calls.

```tspp
outer(middle(inner(value)))
```

```tspp expected
outer(middle(inner(value)));
```

### nested calls that break

When nested calls don't fit, they break appropriately.

```tspp line-width=30
outer(middle(inner(longValue)))
```

```tspp expected
outer(
    middle(inner(longValue)),
);
```

## Deeply Nested Callbacks

### nested callbacks in chain

Deeply nested callbacks within method chains.

```tspp line-width=50
fetch(url).then((res) => res.json()).then((data) => process(data)).catch((err) => handle(err))
```

```tspp expected
fetch(url)
    .then((res) => res.json())
    .then((data) => process(data))
    .catch((err) => handle(err));
```

### callback inside callback

Callbacks passed as arguments to other callbacks.

```tspp line-width=50
outer((x) => inner((y) => transform(x, y)))
```

```tspp expected
outer((x) => inner((y) => transform(x, y)));
```

### deeply nested callback breaks

Very deep nesting breaks appropriately.

```tspp line-width=40
a((x) => b((y) => c((z) => d(x, y, z))))
```

```tspp expected
a((x) =>
    b((y) => c((z) => d(x, y, z))),
);
```

## Curried Function Calls

### curried call

Curried function application.

```tspp
curry(a)(b)(c)
```

```tspp expected
curry(a)(b)(c);
```

### long curried call breaks inner tail first

Long curried calls break the inner tail before the final call when width runs out.

```tspp line-width=30
curriedFunction(firstArg)(secondArg)(thirdArg)
```

```tspp expected
curriedFunction(firstArg)(
    secondArg,
)(thirdArg);
```

### curried call with objects

Curried calls keep simple tails together and break object tails.

```tspp line-width=40
configure({ mode: "dev" })({ debug: true })({ verbose: false })
```

```tspp expected
configure({ mode: "dev" })({
    debug: true,
})({ verbose: false });
```

## Async/Await Patterns

### await call

Await expressions format normally.

```tspp
const data = await fetch(url)
```

```tspp expected
const data = await fetch(url);
```

### await in chain

Await works with method chains.
Long expressions use method chaining style.

```tspp line-width=40
const json = await fetch(url).then((r) => r.json())
```

```tspp expected
const json = await fetch(url).then(
    (r) => r.json(),
);
```

### multiple awaits in expression

Multiple awaits in one expression.

```tspp
const result = await process(await fetch(url))
```

```tspp expected
const result = await process(await fetch(url));
```

### async arrow function

Async arrow functions.

```tspp
const handler = async (event) => await processEvent(event)
```

```tspp expected
const handler = async (event) => await processEvent(event);
```

### async iife

Async immediately invoked function expression.

```tspp
(async () => { const data = await fetch(url); return data })()
```

```tspp expected
(async () => {
    const data = await fetch(url);
    return data;
})();
```
