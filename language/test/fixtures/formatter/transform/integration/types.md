# Type Integration

Type integration fixtures cover interactions between conditional, tuple, predicate, and ownership type syntax.

## Conditional Types

### conditional tuple predicate branch

Tuple inference and predicate function types compose inside conditional branches.

```ds line-width=80
type ExtractRoute<T> = T extends [infer Head, ...infer Tail] ? (value: Head) => value is Tail[number] : never
```

```ds expected
type ExtractRoute<T> = T extends [infer Head, ...infer Tail]
    ? (value: Head) => value is Tail[number]
    : never;
```

### ownership tuple conditional branch

Ownership references inside tuple branches keep their operator grouping.

```ds line-width=80
type BorrowedPair<T> = T extends [infer Left, infer Right] ? [&Left, &readonly Right] : never
```

```ds expected
type BorrowedPair<T> = T extends [infer Left, infer Right]
    ? [&Left, &readonly Right]
    : never;
```
