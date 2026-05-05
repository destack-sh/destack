# Minus

`-` supports receiver overloads, while `-%` and `-|` are builtin overflow-policy operators.

## numbers

### number minus number

> Subtracting two numbers produces a number.

```ds
const value = 5 - 3;
value satisfies 2;
value satisfies int;
value satisfies float64;
value satisfies number;
```

### minus rejects boolean operands

> `-` rejects operand pairs without a numeric or overload rule.

```ds
const value = true - 2;
```

- contains: no matching overload

## overloads

### minus dispatches to Subtract

> `-` dispatches to `Subtract` on the receiver.

```ds
struct Scalar { value: int }

extension of Scalar implements Subtract<Scalar> {
    subtract(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left - right;
value satisfies Scalar;
```

### minus requires Subtract

> `-` requires a matching `Subtract` implementation.

```ds
struct Scalar { value: int }

extension of Scalar implements Add<Scalar> {
    add(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left - right;
```

- contains: no matching overload

## wrapping

### wrapping minus is builtin integer arithmetic

> `-%` wraps modulo the integer range.

```ds
const a: uint8 = 5;
const b: uint8 = 10;

const value = a -% b;
value satisfies uint8;
```

### saturating minus is builtin integer arithmetic

> `-|` clamps to the integer range.

```ds
const a: uint8 = 5;
const b: uint8 = 10;

const value = a -| b;
value satisfies uint8;
```

### wrapping minus rejects user types

> `-%` is not an overloadable operator.

```ds
struct Scalar { value: int }

extension of Scalar implements Subtract<Scalar> {
    subtract(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left -% right;
```

- contains: no matching overload
