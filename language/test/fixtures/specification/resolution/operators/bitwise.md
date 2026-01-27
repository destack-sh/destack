# Bitwise Operators

Bitwise operators work on numeric types.

## elementwise operators

### bitwise and yields the left numeric type

> Elementwise and preserves the left numeric type.

```ds
declare const left: int32;
declare const right: int32;

let value: int32 = left & right;
```

### bitwise or yields the left numeric type

> Elementwise or preserves the left numeric type.

```ds
declare const left: int32;
declare const right: int32;

let value: int32 = left | right;
```

### bitwise xor yields the left numeric type

> Elementwise xor preserves the left numeric type.

```ds
declare const left: int32;
declare const right: int32;

let value: int32 = left ^ right;
```

### bitwise not yields the operand numeric type

> Elementwise not preserves the operand type.

```ds
declare const value: int32;

let out: int32 = ~value;
```

## shifts

### shift left yields the left numeric type

> Shift left preserves the left numeric type.

```ds
declare const left: int32;
declare const right: int32;

let value: int32 = left << right;
```

### shift right yields the left numeric type

> Shift right preserves the left numeric type.

```ds
declare const left: int32;
declare const right: int32;

let value: int32 = left >> right;
```

### unsigned shift right yields the left numeric type

> Unsigned shift right preserves the left numeric type.

```ds
declare const left: uint32;
declare const right: uint32;

let value: uint32 = left >>> right;
```
