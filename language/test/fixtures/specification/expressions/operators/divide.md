# Divide

`/` uses numeric rules for builtin numbers and `Divide` for receiver overloads.

## numbers

### number divided by number

Dividing two numbers produces a number.

```ds
const value = 10 / 2;
value satisfies 5;
value satisfies int;
value satisfies float64;
value satisfies number;
```

## overloads

### divide dispatches to Divide

`/` dispatches to `Divide` on the receiver.

```ds
struct Scalar { value: int }

extension of Scalar implements Divide<Scalar> {
    type Output = Scalar;

    divide(other: Scalar): this.Output { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left / right;
value satisfies Scalar;
```

### divide requires Divide

`/` requires a matching `Divide` implementation.

```ds
struct Scalar { value: int }

extension of Scalar implements Add<Scalar> {
    type Output = Scalar;

    add(other: Scalar): this.Output { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left / right;
```

- contains: no matching overload
