# Conditional Types

## Conditional Types

### conditional with union branches

Conditional types format with spaces around `?` and `:`.

```ds
type Maybe<T> = T extends string ? T | null : T
```

```ds expected
type Maybe<T> = T extends string ? T | null : T;
```

### conditional branch comments

Conditional type branch comments stay attached to their original branch boundaries.

```ds
type Result<T> = T extends string // test-line
  ? // then-line
    StringValue
  : // else-line
    OtherValue
```

```ds expected
type Result<T> = T extends string // test-line
    ? // then-line
      StringValue
    : // else-line
      OtherValue;
```

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
