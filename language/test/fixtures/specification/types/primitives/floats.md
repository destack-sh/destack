# Floats

## precision

### float32 accepts literals

`float32` accepts float literals.

```ds
let value: float32 = 1.5;
```

### float32 widens to float64

`float32` widens to `float64`.

```ds
let small: float32 = 1.5;
let wide: float64 = small;
```

### float64 does not narrow to float32

`float64` does not narrow to `float32` without an explicit conversion.

```ds
let wide: float64 = 1.5;
let narrow: float32 = wide;
```

- contains: not assignable

### floats do not implicitly convert to integers

Float values do not implicitly convert to integer types.

```ds
let value: int32 = 1.5;
```

- contains: not assignable

### float arithmetic stays float-typed

Float arithmetic results remain compatible with float targets.

```ds
let value: float32 = 1.5 + 2.5;
value satisfies float32;
```
