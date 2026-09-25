# Array Spread

## Spread Elements

### array with spread

Spread elements stay tight with the ellipsis.

```tspp
[head, ...rest]
```

```tspp expected
[head, ...rest];
```

### array with multiple spreads

Multiple spread elements keep spacing normalized.

```tspp
[...left, middle, ...right]
```

```tspp expected
[...left, middle, ...right];
```


## Spread

### spread in array

Spread operator expands iterables inline.

```tspp
[...items]
```

```tspp expected
[...items];
```

### spread with elements

Spread can be mixed with regular elements.

```tspp
[1, ...items, 2]
```

```tspp expected
[1, ...items, 2];
```

### multiple spreads

Multiple spreads can appear in one array.

```tspp
[...a, ...b, ...c]
```

```tspp expected
[...a, ...b, ...c];
```

### spread at start

Spread can appear at the beginning.

```tspp
[...prefix, 1, 2, 3]
```

```tspp expected
[...prefix, 1, 2, 3];
```

### spread at end

Spread can appear at the end.

```tspp
[1, 2, 3, ...suffix]
```

```tspp expected
[1, 2, 3, ...suffix];
```
