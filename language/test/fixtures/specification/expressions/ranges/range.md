# Range Basics

## exclusive ranges

### exclusive range expression is allowed

> Exclusive ranges are valid expressions.

```ds
let range = 0..10;
range;
```

## inclusive ranges

### inclusive range expression is allowed

> Inclusive ranges are valid expressions.

```ds
let range = 0..=10;
range;
```

## variable ranges

### range expressions accept variable bounds

> Range bounds can be arbitrary expressions.

```ds
let start = 1;
let end = 5;
let range = start..end;
range;
```

## typing

### exclusive range uses Range type

> Exclusive ranges produce `Range<T>` values.

```ds
let start: int32 = 0;
let end: int32 = 10;
let range: Range<int32> = start..end;
range;
```

### inclusive range uses RangeInclusive type

> Inclusive ranges produce `RangeInclusive<T>` values.

```ds
let start: int32 = 0;
let end: int32 = 10;
let range: RangeInclusive<int32> = start..=end;
range;
```

### range element types must match the target

> Range element types must satisfy the target type.

```ds
let range: Range<string> = 0..10;
range;
```

- contains: not assignable

## iteration

### range expressions can be iterated

> Range expressions can appear in for-of loops.

```ds
for (const i of 0..10) {
    i;
}
```
