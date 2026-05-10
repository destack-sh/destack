# Numeric Literals

## Numeric Literals

### integer

Integer literals are preserved.

```ds
const x = 42
```

```ds expected
const x = 42;
```

### negative integer

Negative numbers use unary minus.

```ds
const x = -42
```

```ds expected
const x = -42;
```

### float

Floating point literals are preserved.

```ds
const x = 3.14
```

```ds expected
const x = 3.14;
```

### scientific notation

Scientific notation is preserved.

```ds
const x = 1e10
```

```ds expected
const x = 1e10;
```

### hexadecimal

Hex literals use `0x` prefix.

```ds
const x = 0xFF
```

```ds expected
const x = 0xff;
```

### octal

Octal literals use `0o` prefix.

```ds
const x = 0o17
```

```ds expected
const x = 0o17;
```

### binary

Binary literals use `0b` prefix.

```ds
const x = 0b1010
```

```ds expected
const x = 0b1010;
```

### bigint

BigInt literals use `n` suffix.

```ds
const x = 42n
```

```ds expected
const x = 42n;
```

### numeric separator

Numeric separators improve readability.

```ds
const x = 1_000_000
```

```ds expected
const x = 1_000_000;
```
