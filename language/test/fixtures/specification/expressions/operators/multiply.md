# Multiply

`*` supports receiver overloads.

## numbers

### number times number

Multiplying two numbers produces a number.

```ds
const value = 2 * 3;
value satisfies 6;
value satisfies int;
value satisfies float64;
value satisfies number;
```

## overloads

### multiply dispatches to Multiply

`*` dispatches to `Multiply` on the receiver.

```ds
struct Scalar {
    value: int;
}

extension of Scalar implements Multiply<Scalar> {
    type Output = Scalar;

    multiply(other: Scalar): this.Output {
        return this;
    }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left * right;
value satisfies Scalar;
```

### multiply requires Multiply

`*` requires a matching `Multiply` implementation.

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

left * right;
```

- contains: no matching overload
