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

## Member Chain Formatting

### short chain stays on one line

Short chains fit on one line.

```ds
obj.method().result
```

```ds expected
obj.method().result;
```

### property access chain

Property access chains stay on one line when possible.

```ds line-width=50
very.long.deeply.nested.property.access
```

```ds expected
very.long.deeply.nested.property.access;
```

### method chain with arguments

Method chains with various argument lengths.

```ds line-width=40
array.filter((x) => x > 0).map((x) => x * 2).reduce((a, b) => a + b, 0)
```

```ds expected
array
    .filter((x) => x > 0)
    .map((x) => x * 2)
    .reduce((a, b) => a + b, 0);
```

### chain starting with call

Chain starting with a function call.

```ds line-width=35
getData().process().transform().result()
```

```ds expected
getData()
    .process()
    .transform()
    .result();
```

### chain with constructor

Chain starting with new expression.

```ds line-width=40
new Builder().setName("test").setAge(25).build()
```

```ds expected
new Builder()
    .setName("test")
    .setAge(25)
    .build();
```

### conditional in method argument

Ternary inside a chained method call.

```ds line-width=50
data.filter((x) => isValid ? x.active : x.pending).map((x) => x.id)
```

```ds expected
data
    .filter((x) => isValid ? x.active : x.pending)
    .map((x) => x.id);
```

### chain with array index

Member chain including array indexing.

```ds line-width=40
users[0].profile.settings.theme
```

```ds expected
users[0].profile.settings.theme;
```

### complex chain with index and call

Mix of property access, indexing, and method calls. Chains break after the receiver.

```ds line-width=35
obj.items[0].getValue().transform()
```

```ds expected
obj
    .items[0]
    .getValue()
    .transform();
```

## Curried Function Calls

### simple curried call

Curried function application.

```ds
curry(a)(b)(c)
```

```ds expected
curry(a)(b)(c);
```

### long curried call breaks

Long curried calls break with each call on its own line.

```ds line-width=30
curriedFunction(firstArg)(secondArg)(thirdArg)
```

```ds expected
curriedFunction(firstArg)
    (secondArg)
    (thirdArg);
```

### curried call with objects

Curried calls with object arguments break similarly.

```ds line-width=40
configure({ mode: "dev" })({ debug: true })({ verbose: false })
```

```ds expected
configure({ mode: "dev" })
    ({ debug: true })
    ({ verbose: false });
```

## Long Binary Expression Patterns

### many additions

Long chain of additions.

```ds line-width=30
const sum = a + b + c + d + e + f + g
```

```ds expected
const sum = a
    + b
    + c
    + d
    + e
    + f
    + g;
```

### mixed logical operators

Chain of mixed && and || operators.

```ds line-width=40
const ok = a && b || c && d || e && f
```

```ds expected
const ok = a && b || c && d || e && f;
```

### nullish chain

Multiple nullish coalescing operators.

```ds line-width=50
const value = first ?? second ?? third ?? fourth ?? fallback
```

```ds expected
const value = first
    ?? second
    ?? third
    ?? fourth
    ?? fallback;
```

### comparison chain with logical

Comparison operators combined with logical.

```ds line-width=35
const inBounds = x >= 0 && x < width && y >= 0 && y < height
```

```ds expected
const inBounds = x >= 0
    && x < width
    && y >= 0
    && y < height;
```

## Advanced Destructuring

### deeply nested object destructuring

Multiple levels of nested object destructuring.

```ds line-width=60
const { user: { profile: { settings: { theme, language } } } } = config
```

```ds expected
const {
    user: { profile: { settings: { theme, language } } },
} = config;
```

### nested destructuring with defaults

Defaults at various nesting levels. Expands when over line width.

```ds line-width=50
const { a: { b = 1, c: { d = 2 } = {} } = {} } = obj
```

```ds expected
const {
    a: { b = 1, c: { d = 2 } = { } } = { },
} = obj;
```

### array destructuring with nested objects

Array elements containing object destructuring expand when needed.

```ds line-width=50
const [{ name, id }, { name: secondName }] = items
```

```ds expected
const [
    { name, id },
    { name: secondName },
] = items;
```

### mixed array and object destructuring

Complex pattern combining arrays and objects.

```ds line-width=60
const { items: [first, { value: secondValue }, ...rest] } = data
```

```ds expected
const {
    items: [first, { value: secondValue }, ...rest],
} = data;
```

### destructuring in function parameters

Destructuring in arrow function parameters. Assignment breaks when too long.

```ds line-width=50
const handler = ({ event: { target, type }, timestamp }) => process(target, type)
```

```ds expected
const handler =
    (
        { event: { target, type }, timestamp },
    ) => process(target, type)
;
```

### rest in nested destructuring

Rest patterns at different levels.

```ds
const { a, ...rest } = obj
const [first, ...remaining] = arr
```

```ds expected
const { a, ...rest } = obj;
const [first, ...remaining] = arr;
```

### _computed property in destructuring

TODO #Incomplete: Destack doesn't support computed property names in destructuring patterns yet.

```ds
const { [key]: value, [prefix + suffix]: other } = obj
```

```ds expected
const { [key]: value, [prefix + suffix]: other } = obj;
```

### destructuring with type annotation

Destructuring with TypeScript-style type annotations.

```ds line-width=60
const { name, age }: { name: string, age: number } = person
```

```ds expected
const { name, age }: { name: string, age: number } = person;
```
