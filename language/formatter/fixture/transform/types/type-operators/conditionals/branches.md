# Conditional Types

## Conditional Types

### conditional with union branches

Conditional types format with spaces around `?` and `:`.

```tspp
type Maybe<T> = T extends string ? T | null : T
```

```tspp expected
type Maybe<T> = T extends string ? T | null : T;
```

### conditional branch comments

Conditional type branch comments stay attached to their original branch boundaries.

```tspp
type Result<T> = T extends string // test-line
  ? // then-line
    StringValue
  : // else-line
    OtherValue
```

```tspp expected
type Result<T> = T extends string // test-line
    ? // then-line
      StringValue
    : // else-line
      OtherValue;
```

### conditional tuple function branch

Tuple inference and function types compose inside conditional branches.

```tspp line-width=80
type ExtractRoute<T> = T extends (infer Head, ...infer Tail) ? (value: Head) => Tail[number] : never
```

```tspp expected
type ExtractRoute<T> = T extends (infer Head, ...infer Tail)
    ? (value: Head) => Tail[number]
    : never;
```

### ownership tuple conditional branch

Ownership references inside tuple branches keep their operator grouping.

```tspp line-width=80
type BorrowedPair<T> = T extends (infer Left, infer Right) ? (&Left, &readonly Right) : never
```

```tspp expected
type BorrowedPair<T> = T extends (infer Left, infer Right)
    ? (&Left, &readonly Right)
    : never;
```

### conditional ownership branch comments

Comments inside ownership-heavy conditional branches stay attached to their branch operands.

```tspp line-width=80
type BorrowedPair<T> = T extends (infer Left, infer Right) ? (& /* left */ Left, &readonly /* right */ Right) : ^ /* moved */ T
```

```tspp expected
type BorrowedPair<T> = T extends (infer Left, infer Right)
    ? (&(/* left */ Left), &readonly (/* right */ Right))
    : ^(/* moved */ T);
```
