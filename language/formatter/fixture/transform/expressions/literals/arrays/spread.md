# Array Spread

## Spread Elements

### array with spread

Spread elements stay tight with the ellipsis.

```ds
[head, ...rest]
```

```ds expected
[head, ...rest];
```

### array with multiple spreads

Multiple spread elements keep spacing normalized.

```ds
[...left, middle, ...right]
```

```ds expected
[...left, middle, ...right];
```


## Spread

### spread in array

Spread operator expands iterables inline.

```ds
[...items]
```

```ds expected
[...items];
```

### spread with elements

Spread can be mixed with regular elements.

```ds
[1, ...items, 2]
```

```ds expected
[1, ...items, 2];
```

### multiple spreads

Multiple spreads can appear in one array.

```ds
[...a, ...b, ...c]
```

```ds expected
[...a, ...b, ...c];
```

### spread at start

Spread can appear at the beginning.

```ds
[...prefix, 1, 2, 3]
```

```ds expected
[...prefix, 1, 2, 3];
```

### spread at end

Spread can appear at the end.

```ds
[1, 2, 3, ...suffix]
```

```ds expected
[1, 2, 3, ...suffix];
```
