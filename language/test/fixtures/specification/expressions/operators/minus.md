# Minus

`-` supports receiver overloads.

## numbers

### number minus number

Subtracting two numbers produces a number.

```ds
const value = 5 - 3;
value satisfies 2;
value satisfies int;
value satisfies float64;
value satisfies number;
```

### minus rejects boolean operands

`-` rejects operand pairs without a numeric or overload rule.

```ds
const value = true - 2;
```

- contains: no matching overload

## overloads

### minus dispatches to Subtract

`-` dispatches to `Subtract` on the receiver.

```ds
struct Scalar {
    value: int;
}

extension of Scalar implements Subtract<Scalar> {
    type Output = Scalar;

    subtract(other: Scalar): this.Output {
        return this;
    }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left - right;
value satisfies Scalar;
```

### minus requires Subtract

`-` requires a matching `Subtract` implementation.

```ds
struct Scalar {
    value: int;
}

extension of Scalar implements Add<Scalar> {
    type Output = Scalar;

    add(other: Scalar): this.Output {
        return this;
    }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left - right;
```

- contains: no matching overload
