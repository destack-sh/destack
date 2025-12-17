# Complex Expression Patterns

Tests for complex, nested, and edge-case expression formatting.

## Deeply Nested Structures

### nested objects in arrays

Deeply nested data structures format with proper indentation.

```ds line-width=40
const data = [{ user: { name: "Alice", settings: { theme: "dark" } } }]
```

```ds expected
const data = [{
    user: {
        name: "Alice",
        settings: { theme: "dark" },
    },
}];
```

### array of arrays of objects

Matrix-like structures with objects break cleanly.

```ds line-width=50
const grid = [[{ x: 0, y: 0 }, { x: 1, y: 0 }], [{ x: 0, y: 1 }, { x: 1, y: 1 }]]
```

```ds expected
const grid = [
    [{ x: 0, y: 0 }, { x: 1, y: 0 }],
    [{ x: 0, y: 1 }, { x: 1, y: 1 }],
];
```

## Chained Operations with Callbacks

### chain with multiple callbacks

Method chains with callback arguments at each step.

```ds line-width=50
data.filter((x) => x.active).map((x) => x.name).reduce((a, b) => a + b)
```

```ds expected
data
    .filter((x) => x.active)
    .map((x) => x.name)
    .reduce((a, b) => a + b);
```

### chain with block callback

When a callback has a block body, the chain continues after the closing brace.

```ds line-width=40
items.map((item) => { return item.value }).filter((v) => v > 0)
```

```ds expected
items.map((item) => {
    return item.value
}).filter((v) => v > 0);
```

### nested chains in callback

Chains inside callback bodies break when they exceed line width.

```ds line-width=50
outer.map((x) => x.items.filter((y) => y.ok).map((y) => y.value))
```

```ds expected
outer.map(
    (x) => x
        .items
        .filter((y) => y.ok)
        .map((y) => y.value),
);
```

## Complex Function Calls

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

## Complex Ternaries

### ternary with function calls

Ternary branches containing function calls.

```ds line-width=50
const result = isValid ? processSuccess(data) : handleFailure(error)
```

```ds expected
const result = isValid
    ? processSuccess(data)
    : handleFailure(error);
```

### ternary with objects

Object literals in ternary branches.

```ds line-width=40
const config = isDev ? { debug: true, log: "verbose" } : { debug: false }
```

```ds expected
const config = isDev
    ? { debug: true, log: "verbose" }
    : { debug: false };
```

### ternary in function argument

Ternary expressions as function arguments.

```ds
render(loading ? <Spinner /> : <Content data={data} />)
```

```ds expected
render(loading ? <Spinner /> : <Content data={data} />);
```

## Binary Expression Chains

### mixed operators same precedence

Operators at the same precedence level flatten together when they don't fit.

```ds line-width=20
const x = a + b + c + d + e
```

```ds expected
const x = a
    + b
    + c
    + d
    + e;
```

### logical chain with calls

Logical operators with function call operands.

```ds line-width=40
const valid = isActive() && hasPermission() && !isBlocked()
```

```ds expected
const valid = isActive()
    && hasPermission()
    && !isBlocked();
```

### comparison chain

Comparison expressions format cleanly.

```ds
const inRange = value >= min && value <= max
```

```ds expected
const inRange = value >= min && value <= max;
```

## Complex Destructuring

### nested object destructuring

Deeply nested destructuring patterns.

```ds
const { user: { profile: { name, avatar } } } = data
```

```ds expected
const { user: { profile: { name, avatar } } } = data;
```

### mixed destructuring with defaults

Destructuring with default values and renaming.

```ds line-width=60
const { name = "default", count: total = 0, items: [...rest] } = config
```

```ds expected
const {
    name = "default",
    count: total = 0,
    items: [...rest],
} = config;
```

### array destructuring with rest

Array destructuring with rest patterns.

```ds
const [first, second, ...remaining] = items
```

```ds expected
const [first, second, ...remaining] = items;
```

## Edge Cases

### empty structures

Empty objects keep internal spacing, arrays and calls are compact.

```ds
const empty = { }
const arr = [   ]
const call = foo(   )
```

```ds expected
const empty = { };
const arr = [];
const call = foo();
```

### single element with trailing comma preserved

Trailing commas in source are normalized.

```ds
const arr = [1,]
const obj = { a: 1, }
```

```ds expected
const arr = [1];
const obj = { a: 1 };
```

### spread in various contexts

Spread operator in different positions.

```ds
const merged = { ...defaults, ...overrides, extra: true }
const combined = [...first, middle, ...last]
fn(...args, extra)
```

```ds expected
const merged = { ...defaults, ...overrides, extra: true };
const combined = [...first, middle, ...last];
fn(...args, extra);
```

### computed property names

Computed property names in objects.

```ds
const obj = { [key]: value, [`prefix_${name}`]: data }
```

```ds expected
const obj = { [key]: value, [`prefix_${name}`]: data };
```

### optional chaining cascade

Multiple optional chaining operators.

```ds
const value = obj?.nested?.deeply?.value
```

```ds expected
const value = obj?.nested?.deeply?.value;
```

### nullish coalescing

Nullish coalescing with fallback chain.

```ds
const result = primary ?? secondary ?? fallback
```

```ds expected
const result = primary ?? secondary ?? fallback;
```

## Chained Assignments

### simple chained assignment

Multiple assignments in one expression.

```ds
a = b = c = 1
```

```ds expected
a = b = c = 1;
```

### long chained assignment breaks

When chained assignments exceed line width, each breaks at same indent level.

```ds line-width=30
veryLongName = anotherLongName = thirdLongName = 42
```

```ds expected
veryLongName =
    anotherLongName =
    thirdLongName =
    42;
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
a(
    (x) => b(
        (y) => c((z) => d(x, y, z)),
    ),
);
```

## Async/Await Patterns

### simple await

Await expressions format normally.

```ds
const data = await fetch(url)
```

```ds expected
const data = await fetch(url);
```

### await in chain

Await works with method chains. Long expressions use method chaining style.

```ds line-width=40
const json = await fetch(url).then((r) => r.json())
```

```ds expected
const json =
    await fetch(url)
        .then((r) => r.json())
;
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
    return data
})();
```

## Mixed Operators

### arithmetic with different precedence

Mixed arithmetic operators respect precedence.

```ds
const x = a + b * c - d / e
```

```ds expected
const x = a + b * c - d / e;
```

### logical with comparison

Logical operators with comparisons.

```ds
const valid = x > 0 && x < 100 || y === 0
```

```ds expected
const valid = x > 0 && x < 100 || y === 0;
```

### long mixed expression breaks

Long expressions with mixed operators break appropriately.

```ds line-width=30
const x = veryLongA + veryLongB * veryLongC
```

```ds expected
const x = veryLongA
    + veryLongB * veryLongC;
```

## Compound Patterns

### optional chain with nullish coalescing

Optional chaining combined with nullish coalescing.

```ds
const name = user?.profile?.name ?? "Anonymous"
```

```ds expected
const name = user?.profile?.name ?? "Anonymous";
```

### optional chain with method call

Optional chaining with method invocation.

```ds
const result = obj?.method?.(arg1, arg2)
```

```ds expected
const result = obj?.method?.(arg1, arg2);
```

### complex optional access

Multiple optional accesses and calls.

```ds
data?.items?.[0]?.value?.toString()
```

```ds expected
data?.items?.[0]?.value?.toString();
```
