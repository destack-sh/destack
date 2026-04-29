# Expression Integration

Expression integration fixtures cover mixed expression forms across formatter families.

## Deeply Nested Structures

### nested objects in arrays

Deeply nested data structures expand with stable indentation.

```ds line-width=40
const data = [{ user: { name: "Alice", settings: { theme: "dark" } } }]
```

```ds expected
const data = [
    {
        user: {
            name: "Alice",
            settings: { theme: "dark" },
        },
    },
];
```

### array of arrays of objects

Nested array rows with objects break cleanly.

```ds line-width=50
const grid = [[{ x: 0, y: 0 }, { x: 1, y: 0 }], [{ x: 0, y: 1 }, { x: 1, y: 1 }]]
```

```ds expected
const grid = [
    [
        { x: 0, y: 0 },
        { x: 1, y: 0 },
    ],
    [
        { x: 0, y: 1 },
        { x: 1, y: 1 },
    ],
];
```

## Structure Boundaries

### empty structures

Empty objects stay compact, arrays and calls are compact.

```ds
const empty = { }
const arr = [   ]
const call = foo(   )
```

```ds expected
const empty = {};
const arr = [];
const call = foo();
```

### blank lines between elements

Blank lines between array and object elements are preserved.

```ds
const arr = [
    1,

    2,

    3,
]
```

```ds expected
const arr = [
    1,

    2,

    3,
];
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

## Sequence Expressions

### unicode sequence in computed index

Sequence expressions inside computed indices are parenthesized.

```ts:main.ts
x = y['x・', 'x･']
```

```ts expected
x = y[("x・", "x･")];
```

### sequence expression statement

Sequence expressions format without extra parentheses in statements.

```ts:main.ts
a, b
```

```ts expected
(a, b);
```
