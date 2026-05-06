# Number Literals

Number literal type inference and checking.

## integer literals

### integer literal

Integer literals can be assigned to number type.

```ds
const x: number = 42;
```

### negative integer

Negative integers are accepted number literals.

```ds
const x: number = -42;
```

### zero

Zero is an accepted number literal.

```ds
const x: number = 0;
```

### hex literal

Hex literals are accepted number literals.

```ds
const x: number = 0xff;
```

### octal literal

Octal literals are accepted number literals.

```ds
const x: number = 0o17;
```

### binary literal

Binary literals are accepted number literals.

```ds
const x: number = 0b1010;
```

## float literals

### float literal

Float literals can be assigned to number type.

```ds
const x: number = 3.14;
```

### negative float

Negative floats are accepted number literals.

```ds
const x: number = -3.14;
```

## inference

### integer literals stay exact

Integer literals without annotation keep their literal value and satisfy wider numeric types.

```ds
const x = 123;
x satisfies 123;
x satisfies number;
```

### float literals stay exact

Float literals without annotation keep their literal value and satisfy wider numeric types.

```ds
const x = 3.14;
x satisfies 3.14;
x satisfies number;
```

### number does not satisfy string

Number literal cannot satisfy string type.

```ds
const x = 123;
x satisfies string;
```

- contains: not assignable

## number members

### number toFixed resolves

Number literals expose Number standard members.


```ds
const value = 12;
const fixed = value.toFixed(2);
fixed satisfies string;
```
