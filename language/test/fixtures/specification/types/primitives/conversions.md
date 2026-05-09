# Conversions

Implicit numeric conversions only widen.

## integer widening

### int8 to int16

Smaller signed integers can be assigned to larger signed integers.

```ds
function takeInt16(value: int16): void {}
declare const smallInteger: int8;
takeInt16(smallInteger);
```

### int32 to int64

`int32` can widen to `int64`.

```ds
function takeInt64(value: int64): void {}
declare const mediumInteger: int32;
takeInt64(mediumInteger);
```

### int16 to int8 rejects narrowing

Larger integers cannot narrow to smaller integers.

```ds
function takeInt8(value: int8): void {}
declare const mediumInteger: int16;
takeInt8(mediumInteger);
```

- contains: not assignable

### uint8 to int16

Unsigned integers can widen to larger signed integers.

```ds
function takeInt16(value: int16): void {}
declare const smallUnsigned: uint8;
takeInt16(smallUnsigned);
```

### int8 to uint8 rejects sign changes

Signed integers cannot widen to unsigned integer types.

```ds
function takeUint8(value: uint8): void {}
declare const signedInteger: int8;
takeUint8(signedInteger);
```

- contains: not assignable

## float widening

### float32 to float64

Smaller floats can widen to larger floats.

```ds
function takeFloat64(value: float64): void {}
declare const smallFloat: float32;
takeFloat64(smallFloat);
```

### float64 to float32 rejects narrowing

Larger floats cannot narrow to smaller floats.

```ds
function takeFloat32(value: float32): void {}
declare const largeFloat: float64;
takeFloat32(largeFloat);
```

- contains: not assignable

## int to float widening

### int32 to float64

Integers can widen to floats.

```ds
function takeFloat64(value: float64): void {}
declare const integer: int32;
takeFloat64(integer);
```

## widening to number

### int32 to number

Integer types can widen to `number`.

```ds
function takeNumber(value: number): void {}
declare const integer: int32;
takeNumber(integer);
```

### float64 to number

Concrete float types can widen to `number`.

```ds
function takeNumber(value: number): void {}
declare const floatValue: float64;
takeNumber(floatValue);
```
