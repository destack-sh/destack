# Range Basics

## exclusive ranges

### exclusive range expression is allowed

> Exclusive ranges are valid expressions.

```ds libs=std
let range = 0..10;
range;
```

## inclusive ranges

### inclusive range expression is allowed

> Inclusive ranges are valid expressions.

```ds libs=std
let range = 0..=10;
range;
```

## variable ranges

### range expressions accept variable bounds

> Range bounds can be arbitrary expressions.

```ds libs=std
let start = 1;
let end = 5;
let range = start..end;
range;
```

## typing

### exclusive range uses Range type

> Exclusive ranges produce `Range<T>` values.

```ds libs=std
let start: int32 = 0;
let end: int32 = 10;
let range: Range<int32> = start..end;
range;
```

### inclusive range uses RangeInclusive type

> Inclusive ranges produce `RangeInclusive<T>` values.

```ds libs=std
let start: int32 = 0;
let end: int32 = 10;
let range: RangeInclusive<int32> = start..=end;
range;
```

### range element types must match the target

> Range element types must satisfy the target type.

```ds libs=std
let range: Range<string> = 0..10;
range;
```

- contains: not assignable

## iteration

### range expressions can be iterated

> Range expressions can appear in for-of loops.

```ds libs=std
for (const i of 0..10) {
    i;
}
```

### range expressions are assignable to iterable

> Range expressions should be assignable to `Iterable<T>`.

```ds libs=std
let range = 0..10;
let iter: Iterable<int32, unknown, unknown> = range;
iter;
```

### inclusive ranges implement RangeBounds

> Range expressions should satisfy `RangeBounds<T>`.

```ds libs=std
let range = 1..=3;
let bounds: RangeBounds<int32> = range;
bounds;
```
