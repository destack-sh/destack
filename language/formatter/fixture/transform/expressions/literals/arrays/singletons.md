# Single Element Arrays

## Single Element Expansion

### single object in array expands outer array

When an array contains a single object that expands, the outer array also expands.

```tspp line-width=20
[{ a: 1, b: 2, c: 3 }]
```

```tspp expected
[
    {
        a: 1,
        b: 2,
        c: 3,
    },
];
```

### single array in array expands outer array

Nested arrays also expand when the single nested array element expands.

```tspp line-width=20
[[1, 2, 3, 4, 5, 6]]
```

```tspp expected
[
    [
        1, 2, 3, 4,
        5, 6,
    ],
];
```
