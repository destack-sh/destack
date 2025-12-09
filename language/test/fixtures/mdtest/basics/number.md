# Number Literals

Tests for number literal type inference and checking.

## Integer Literals

### integer literal

> Integer literals can be assigned to number type.

```ds
const x: number = 42
```

### negative integer

> Negative integers are valid number literals.

```ds
const x: number = -42
```

### zero

> Zero is a valid number literal.

```ds
const x: number = 0
```

## Float Literals

### float literal

> Float literals can be assigned to number type.

```ds
const x: number = 3.14
```

### negative float

> Negative floats are valid number literals.

```ds
const x: number = -3.14
```

## Inference

### inferred integer type

> Integer literals without annotation infer to number.

```ds
const x = 123
```

### inferred float type

> Float literals without annotation infer to number.

```ds
const x = 3.14
```
