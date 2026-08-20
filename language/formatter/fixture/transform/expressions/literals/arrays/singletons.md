# Single Element Arrays

## Single Element Expansion

### single object in array expands outer array

When an array contains a single object that expands, the outer array also expands.

```ds line-width=20
[{ a: 1, b: 2, c: 3 }]
```

```ds expected
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

```ds line-width=20
[[1, 2, 3, 4, 5, 6]]
```

```ds expected
[
    [
        1, 2, 3, 4,
        5, 6,
    ],
];
```
