# Arithmetic Operators

Tests for arithmetic binary operators.

## Addition

### number plus number

> Adding two numbers produces a number.

```ds
const x = 1 + 2;
x satisfies 3;
x satisfies int;
x satisfies float;
x satisfies number;
```

## Subtraction

### number minus number

> Subtracting two numbers produces a number.

```ds
const x = 5 - 3;
x satisfies 2;
x satisfies int;
x satisfies float;
x satisfies number;
```

## Multiplication

### number times number

> Multiplying two numbers produces a number.

```ds
const x = 2 * 3;
x satisfies 6;
x satisfies int;
x satisfies float;
x satisfies number;
```

## Division

### number divided by number

> Dividing two numbers produces a number.

```ds
const x = 10 / 2;
x satisfies 5;
x satisfies int;
x satisfies float;
x satisfies number;
```

## mixed operands

### arithmetic operators reject incompatible operand types

> Arithmetic operators reject incompatible operand type pairs.

```ds
const value = 1 + "two";
```

- contains: no matching overload

### arithmetic operators reject boolean operands

> Arithmetic operators reject boolean operands.

```ds
const value = true * 2;
```

- contains: no matching overload
