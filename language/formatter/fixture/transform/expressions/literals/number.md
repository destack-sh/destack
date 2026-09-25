# Numeric Literals

## Numeric Literals

### integer

Integer literals are preserved.

```tspp
const x = 42
```

```tspp expected
const x = 42;
```

### negative integer

Negative numbers use unary minus.

```tspp
const x = -42
```

```tspp expected
const x = -42;
```

### float

Floating point literals are preserved.

```tspp
const x = 3.14
```

```tspp expected
const x = 3.14;
```

### scientific notation

Scientific notation is preserved.

```tspp
const x = 1e10
```

```tspp expected
const x = 1e10;
```

### hexadecimal

Hex literals use `0x` prefix.

```tspp
const x = 0xFF
```

```tspp expected
const x = 0xff;
```

### octal

Octal literals use `0o` prefix.

```tspp
const x = 0o17
```

```tspp expected
const x = 0o17;
```

### binary

Binary literals use `0b` prefix.

```tspp
const x = 0b1010
```

```tspp expected
const x = 0b1010;
```

### bigint

BigInt literals use `n` suffix.

```tspp
const x = 42n
```

```tspp expected
const x = 42n;
```

### numeric separator

Numeric separators improve readability.

```tspp
const x = 1_000_000
```

```tspp expected
const x = 1_000_000;
```
