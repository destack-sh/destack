# Precise Primitives

> NOTE #Incomplete: implement/mdtest precise primitives

Precise numeric types beyond TypeScript's `number`.

## Coverage

### Integers

- **Signed**: `int8`, `int16`, `int32`, `int64`, `int128`
- **Unsigned**: `uint8`, `uint16`, `uint32`, `uint64`, `uint128`
- **Aliases**: `int` = `int64`, `uint` = `uint64`
- **Arbitrary width**: `int3`, `uint17`, etc.

### Floats

- **Sizes**: `float32`, `float64`
- **Aliases**: `float` = `float64`, `number` = `float64`

### Characters

- **Character**: Single Unicode codepoint

## Example

```ds
const id: uint64 = 12345;
const balance: float32 = 100.50;
const small: int8 = 127;
```
