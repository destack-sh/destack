# Numeric Widening

Tests for implicit numeric type widening.

## Integer Widening

### int8 to int16

> Smaller signed integers can be assigned to larger signed integers.

```ds
function takeInt16(x: int16): void {}
declare const small: int8;
takeInt16(small);
```

### int32 to int64

> int32 can widen to int64.

```ds
function takeInt64(x: int64): void {}
declare const medium: int32;
takeInt64(medium);
```

### int16 to int8 fails

> Larger integers cannot narrow to smaller integers.

```ds
function takeInt8(x: int8): void {}
declare const medium: int16;
takeInt8(medium);
```

- type int16 is not assignable to type int8

### uint8 to int16

> Unsigned integers can widen to larger signed integers.

```ds
function takeInt16(x: int16): void {}
declare const small: uint8;
takeInt16(small);
```

### int8 to uint8 fails

> Signed integers cannot widen to unsigned (may lose negative values).

```ds
function take_uint8(x: uint8): void {}
declare const signed: int8;
take_uint8(signed);
```

- type int8 is not assignable to type uint8

## Float Widening

### float32 to float64

> Smaller floats can widen to larger floats.

```ds
function take_float64(x: float64): void {}
declare const small: float32;
take_float64(small);
```

### float64 to float32 fails

> Larger floats cannot narrow to smaller floats.

```ds
function take_float32(x: float32): void {}
declare const large: float64;
take_float32(large);
```

- type float64 is not assignable to type float32

## Int to Float Widening

### int32 to float64

> Integers can widen to floats.

```ds
function take_float64(x: float64): void {}
declare const i: int32;
take_float64(i);
```

## Widening to Number

### int32 to number

> Any integer type can widen to number.

```ds
function take_number(x: number): void {}
declare const i: int32;
take_number(i);
```

### float64 to number

> Any float type can widen to number.

```ds
function take_number(x: number): void {}
declare const f: float64;
take_number(f);
```
