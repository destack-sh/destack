# Array Forms

## Spacing

### array brackets have no internal spacing

Spaces after `[` and before `]` should be removed.

```tspp
[ 1, 2, 3 ]
```

```tspp expected
[1, 2, 3];
```

### short arrays stay on one line

Short array literals remain on a single line.

```tspp
[1, 2, 3, 4, 5]
```

```tspp expected
[1, 2, 3, 4, 5];
```

### empty array

Empty arrays have all spaces removed.

```tspp
[   ]
```

```tspp expected
[];
```

### single element array

Single elements have internal spacing removed.

```tspp
[   1   ]
```

```tspp expected
[1];
```

### array normalizes comma spacing

Commas get a space after but not before.

```tspp
[1,2,3]
```

```tspp expected
[1, 2, 3];
```

### array removes extra spaces

Extra spaces around elements are normalized.

```tspp
[  1  ,  2  ,  3  ]
```

```tspp expected
[1, 2, 3];
```

## Trailing Comma

### short array without trailing comma

Short arrays on one line don't get trailing commas.

```tspp
[1, 2, 3]
```

```tspp expected
[1, 2, 3];
```

### single element with trailing comma

Trailing commas in source are normalized.

```tspp
const arr = [1,]
```

```tspp expected
const arr = [1];
```

### blank lines between elements

Blank lines between array elements are preserved.

```tspp
const arr = [
    1,

    2,

    3,
]
```

```tspp expected
const arr = [
    1,

    2,

    3,
];
```

## Comments

### array element sibling comments

Trailing element comments and next element leading comments keep separate ownership.

```tspp
const value = [first, // first
// second
second]
```

```tspp expected
const value = [
    first, // first
    // second
    second,
];
```
