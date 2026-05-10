# Array Type Annotations

## Type Annotations

### typed array

Array type annotations use `T[]` syntax.

```ds
const x: number[] = [1, 2, 3]
```

```ds expected
const x: number[] = [1, 2, 3];
```

### array with as

`as const` makes array literal readonly.

```ds
[1, 2, 3] as const
```

```ds expected
[1, 2, 3] as const;
```

### array satisfies type

`satisfies` checks type without changing it.

```ds
[1, 2, 3] satisfies number[]
```

```ds expected
[1, 2, 3] satisfies number[];
```

## Type Assertions

### array as const

Const assertions stay on the same line as the array literal.

```ts:main.ts
const values = [1, 2, 3] as const
```

```ts expected
const values = [1, 2, 3] as const;
```
