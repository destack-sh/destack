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
} =
    config
;
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
