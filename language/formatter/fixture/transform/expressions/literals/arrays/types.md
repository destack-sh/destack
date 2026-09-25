# Array Type Annotations

## Type Annotations

### typed array

Array type annotations use `T[]` syntax.

```tspp
const x: number[] = [1, 2, 3]
```

```tspp expected
const x: number[] = [1, 2, 3];
```

### array with as

`as const` makes array literal readonly.

```tspp
[1, 2, 3] as const
```

```tspp expected
[1, 2, 3] as const;
```

### array satisfies type

`satisfies` checks type without changing it.

```tspp
[1, 2, 3] satisfies number[]
```

```tspp expected
[1, 2, 3] satisfies number[];
```

## Type Assertions

### array as const

Const assertions stay on the same line as the array literal.

```tspp:main.tspp
const values = [1, 2, 3] as const
```

```tspp expected
const values = [1, 2, 3] as const;
```
