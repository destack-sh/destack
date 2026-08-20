# Array Forms

## Spacing

### array brackets have no internal spacing

Spaces after `[` and before `]` should be removed.

```ds
[ 1, 2, 3 ]
```

```ds expected
[1, 2, 3];
```

### short arrays stay on one line

Short array literals remain on a single line.

```ds
[1, 2, 3, 4, 5]
```

```ds expected
[1, 2, 3, 4, 5];
```

### empty array

Empty arrays have all spaces removed.

```ds
[   ]
```

```ds expected
[];
```

### single element array

Single elements have internal spacing removed.

```ds
[   1   ]
```

```ds expected
[1];
```

### array normalizes comma spacing

Commas get a space after but not before.

```ds
[1,2,3]
```

```ds expected
[1, 2, 3];
```

### array removes extra spaces

Extra spaces around elements are normalized.

```ds
[  1  ,  2  ,  3  ]
```

```ds expected
[1, 2, 3];
```

## Trailing Comma

### short array without trailing comma

Short arrays on one line don't get trailing commas.

```ds
[1, 2, 3]
```

```ds expected
[1, 2, 3];
```

### single element with trailing comma

Trailing commas in source are normalized.

```ds
const arr = [1,]
```

```ds expected
const arr = [1];
```

### blank lines between elements

Blank lines between array elements are preserved.

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

## Comments

### array element sibling comments

Trailing element comments and next element leading comments keep separate ownership.

```ds
const value = [first, // first
// second
second]
```

```ds expected
const value = [
    first, // first
    // second
    second,
];
```
