# Nested Arrays

## Nested Arrays

### nested array

Nested arrays break with one nested element per line.

```tspp
[[1, 2], [3, 4]]
```

```tspp expected
[
    [1, 2],
    [3, 4],
];
```

### deeply nested array

Any depth of nesting is preserved.

```tspp
[[[1]]]
```

```tspp expected
[[[1]]];
```

### nested array breaks at line width

Outer array breaks while inner arrays stay compact.

```tspp line-width=20
[[1, 2, 3], [4, 5, 6], [7, 8, 9]]
```

```tspp expected
[
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
];
```

### nested rows

Nested arrays format with one row per line.

```tspp line-width=30
[[1, 0, 0], [0, 1, 0], [0, 0, 1]]
```

```tspp expected
[
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
];
```

### array of objects breaks

Object elements break to one per line when they exceed width.

```tspp line-width=30
[{ a: 1, b: 2 }, { c: 3, d: 4 }]
```

```tspp expected
[
    { a: 1, b: 2 },
    { c: 3, d: 4 },
];
```

## Array of Objects

### array of objects stays on one line if short

Short object elements stay inline.

```tspp
[{ a: 1 }, { a: 2 }]
```

```tspp expected
[{ a: 1 }, { a: 2 }];
```

### array of complex objects

Complex objects break to separate lines.

```tspp line-width=40
[{ id: 1, name: "first" }, { id: 2, name: "second" }]
```

```tspp expected
[
    { id: 1, name: "first" },
    { id: 2, name: "second" },
];
```

### nested objects in arrays

Deeply nested data structures expand with stable indentation.

```tspp line-width=40
const data = [{ user: { name: "Alice", settings: { theme: "dark" } } }]
```

```tspp expected
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

```tspp line-width=50
const grid = [[{ x: 0, y: 0 }, { x: 1, y: 0 }], [{ x: 0, y: 1 }, { x: 1, y: 1 }]]
```

```tspp expected
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
