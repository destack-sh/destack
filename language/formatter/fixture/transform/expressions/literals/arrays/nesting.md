# Nested Arrays

## Nested Arrays

### nested array

Nested arrays break with one nested element per line.

```ds
[[1, 2], [3, 4]]
```

```ds expected
[
    [1, 2],
    [3, 4],
];
```

### deeply nested array

Any depth of nesting is preserved.

```ds
[[[1]]]
```

```ds expected
[[[1]]];
```

### nested array breaks at line width

Outer array breaks while inner arrays stay compact.

```ds line-width=20
[[1, 2, 3], [4, 5, 6], [7, 8, 9]]
```

```ds expected
[
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
];
```

### nested rows

Nested arrays format with one row per line.

```ds line-width=30
[[1, 0, 0], [0, 1, 0], [0, 0, 1]]
```

```ds expected
[
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
];
```

### array of objects breaks

Object elements break to one per line when they exceed width.

```ds line-width=30
[{ a: 1, b: 2 }, { c: 3, d: 4 }]
```

```ds expected
[
    { a: 1, b: 2 },
    { c: 3, d: 4 },
];
```

## Array of Objects

### array of objects stays on one line if short

Short object elements stay inline.

```ds
[{ a: 1 }, { a: 2 }]
```

```ds expected
[{ a: 1 }, { a: 2 }];
```

### array of complex objects

Complex objects break to separate lines.

```ds line-width=40
[{ id: 1, name: "first" }, { id: 2, name: "second" }]
```

```ds expected
[
    { id: 1, name: "first" },
    { id: 2, name: "second" },
];
```

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
