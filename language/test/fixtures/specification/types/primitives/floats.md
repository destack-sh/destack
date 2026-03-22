# Arbitrary width floats

## tests

### floatN accepts literals

> Arbitrary width floats accept float literals.

```ds
let value: float16 = 1.5;
```

### floatN widens to larger floats

> Smaller floats widen to larger float widths.

```ds
let small: float16 = 1.5;
let wide: float32 = small;
```

### floatN does not narrow from larger floats

> Larger floats do not narrow without an explicit conversion.

```ds
let wide: float32 = 1.5;
let narrow: float16 = wide;
```

- not assignable

### floatN does not implicitly convert to integers

> Float values do not implicitly convert to integer types.

```ds
let value: int32 = 1.5;
```

- not assignable

### floatN values can widen through multiple widths

> Float values can widen transitively through increasing widths.

```ds
let small: float16 = 1.5;
let medium: float32 = small;
let large: float64 = medium;
large satisfies float64;
```

### floatN arithmetic keeps float compatibility

> Float arithmetic results remain compatible with float targets.

```ds
let value: float32 = 1.5 + 2.5;
value satisfies float32;
```
