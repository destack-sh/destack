# Number Literals

Number literal type inference and checking.

## Integer Literals

### integer literal

> Integer literals can be assigned to number type.

```ds
const x: number = 42;
```

### negative integer

> Negative integers are valid number literals.

```ds
const x: number = -42;
```

### zero

> Zero is a valid number literal.

```ds
const x: number = 0;
```

### hex literal

> Hex literals are valid number literals.

```ds
const x: number = 0xff;
```

### octal literal

> Octal literals are valid number literals.

```ds
const x: number = 0o17;
```

### binary literal

> Binary literals are valid number literals.

```ds
const x: number = 0b1010;
```

## Float Literals

### float literal

> Float literals can be assigned to number type.

```ds
const x: number = 3.14;
```

### negative float

> Negative floats are valid number literals.

```ds
const x: number = -3.14;
```

## Inference

### inferred integer type

> Integer literals without annotation infer to number.

```ds
const x = 123;
x satisfies number;
```

### inferred float type

> Float literals without annotation infer to number.

```ds
const x = 3.14;
x satisfies number;
```

### number does not satisfy string

> Number literal cannot satisfy string type.

```ds
const x = 123;
x satisfies string;
```

- contains: expected string, found 123

## Number Members

### number toFixed resolves

> Number literals expose Number standard members.


```ds libs=es5
const value = 12;
const fixed = value.toFixed(2);
fixed satisfies string;
```
