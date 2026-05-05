# Power

`**` uses numeric rules for builtin numbers and `Power` for receiver overloads.

## numbers

### number power number

> Raising one number to another produces a number.

```ds
const value = 2 ** 3;
value satisfies 8;
value satisfies int;
value satisfies float64;
value satisfies number;
```

## overloads

### power dispatches to Power

> `**` dispatches to `Power` on the receiver.

```ds
struct Scalar { value: int }

extension of Scalar implements Power<Scalar> {
    power(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left ** right;
value satisfies Scalar;
```

### power requires Power

> `**` requires a matching `Power` implementation.

```ds
struct Scalar { value: int }

extension of Scalar implements Multiply<Scalar> {
    multiply(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left ** right;
```

- contains: no matching overload
