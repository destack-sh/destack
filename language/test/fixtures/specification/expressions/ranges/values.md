# Range Values

Range expressions produce range values.
Range values are used by slicing, indexing, iteration, and APIs that store bounds.

## bounds

### half-open ranges

`..` excludes the end.

```ds
const range = 0..10;
range satisfies Range<int>;
```

### inclusive ranges

`..=` includes the end.

```ds
const range = 0..=10;
range satisfies RangeInclusive<int>;
```

### one-sided ranges

Either endpoint can be omitted.

```ds
const from = 2..;
const to = ..10;
const through = ..=10;

from satisfies RangeFrom<int>;
to satisfies RangeTo<int>;
through satisfies RangeToInclusive<int>;
```

### full ranges

`..` alone spans everything.

```ds
const range = ..;
range satisfies RangeFull;
```

## typing

### endpoints must share one type

Mixed endpoint types do not unify.

```ds
const range = 0..10n;
```

- contains: no matching overload

### inclusive ranges keep their endpoint value

`..=` can name the maximum endpoint value without overflowing.

```ds
const last: uint8 = 255;
const range = 0..=last;

range satisfies RangeInclusive<uint8>;
```

### float endpoints are allowed for range values

Range values accept any scalar; only iteration requires `Step`.

```ds
const range = 0.0..1.0;

range satisfies Range<float>;
```
