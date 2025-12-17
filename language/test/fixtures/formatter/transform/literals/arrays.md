# Array Literals

Tests for array literal formatting.

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

## Line Breaking

### array breaks when line width is exceeded

When the line width is exceeded, arrays break to multiple lines.

```ds line-width=10
[1, 2, 3, 4, 5]
```

```ds expected
[
    1,
    2,
    3,
    4,
    5,
];
```
