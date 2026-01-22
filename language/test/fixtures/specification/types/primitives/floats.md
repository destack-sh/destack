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

- contains: not assignable
