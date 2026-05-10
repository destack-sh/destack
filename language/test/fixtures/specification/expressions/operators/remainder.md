# Remainder

`%` uses numeric rules for builtin numbers and `Remainder` for receiver overloads.

## numbers

### number remainder number

Remainder of two numbers produces a number.

```ds
const value = 10 % 3;
value satisfies 1;
value satisfies int;
value satisfies float64;
value satisfies number;
```

## overloads

### remainder dispatches to Remainder

`%` dispatches to `Remainder` on the receiver.

```ds
struct Scalar {
    value: int;
}

extension of Scalar implements Remainder<Scalar> {
    type Output = Scalar;

    remainder(other: Scalar): this.Output {
        return this;
    }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left % right;
value satisfies Scalar;
```

### remainder requires Remainder

`%` requires a matching `Remainder` implementation.

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

left % right;
```

- contains: no matching overload
