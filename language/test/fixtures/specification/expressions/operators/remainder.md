# Remainder

`%` uses numeric rules for builtin numbers and `Remainder` for receiver overloads.

## numbers

### number remainder number

> Remainder of two numbers produces a number.

```ds
const value = 10 % 3;
value satisfies 1;
value satisfies int;
value satisfies float;
value satisfies number;
```

## overloads

### remainder dispatches to Remainder

> `%` dispatches to `Remainder` on the receiver.

```ds
struct Scalar { value: int }

extension of Scalar implements Remainder<Scalar> {
    remainder(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left % right;
value satisfies Scalar;
```
