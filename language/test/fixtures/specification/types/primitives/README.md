# Precise Primitives

Precise numeric types beyond TypeScript's `number`.

## Coverage

### Integers

- **Signed**: `int8`, `int16`, `int32`, `int64`, `int128`
- **Unsigned**: `uint8`, `uint16`, `uint32`, `uint64`, `uint128`
- **Pointer-sized**: `isize`, `usize`
- **Defaults**: `int` and `uint` use the compiler's default integer width (default 32-bit).
- **Arbitrary width**: `int3`, `uint17`, etc.

### Floats

- **Sizes**: `float32`, `float64`, arbitrary widths (`float16`, `float128`, etc.)
- **Defaults**: `float` uses the compiler's default float width (default 64-bit).
- **Number**: `number` is the JS-compatible numeric supertype.

### Characters

- **Character**: Single Unicode codepoint

## Example

```ds
const id: uint64 = 12345;
const balance: float32 = 100.50;
const small: int8 = 127;
```

Primitive behavior coverage lives in the sibling fixtures in this directory.
