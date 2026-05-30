# Floats

## precision

### float16 accepts literals

`float16` accepts representable float literals.

```ds
let value: float16 = 1.5;
value satisfies float16;
```

### bfloat16 accepts literals

`bfloat16` accepts representable float literals.

```ds
let value: bfloat16 = 1.5;
value satisfies bfloat16;
```

### float32 accepts literals

`float32` accepts float literals.

```ds
let value: float32 = 1.5;
```

### float16 widens to float32

`float16` widens to `float32`.

```ds
let small: float16 = 1.5;
let wide: float32 = small;
```

### bfloat16 widens to float32

`bfloat16` widens to `float32`.

```ds
let small: bfloat16 = 1.5;
let wide: float32 = small;
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

### same width float formats do not implicitly convert

`float16` and `bfloat16` have distinct concrete formats.

```ds
let half: float16 = 1.5;
let brain: bfloat16 = half;
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
