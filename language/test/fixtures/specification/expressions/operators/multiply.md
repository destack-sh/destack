# Multiply

`*`, `*%`, and `*|` use numeric rules for builtin numbers and `Multiply` for receiver overloads.

## numbers

### number times number

> Multiplying two numbers produces a number.

```ds
const value = 2 * 3;
value satisfies 6;
value satisfies int;
value satisfies float;
value satisfies number;
```

## overloads

### multiply dispatches to Multiply

> `*` dispatches to `Multiply` on the receiver.

```ds
struct Scalar { value: int }

extension of Scalar implements Multiply<Scalar> {
    multiply(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left * right;
value satisfies Scalar;
```

### multiply requires Multiply

> `*` requires a matching `Multiply` implementation.

```ds
struct Scalar { value: int }

extension of Scalar implements Add<Scalar> {
    add(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

left * right;
```

- contains: no matching overload

## wrapping

### wrapping multiply uses Multiply

> `*%` uses the same receiver contract as `*`.

```ds
struct Scalar { value: int }

extension of Scalar implements Multiply<Scalar> {
    multiply(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left *% right;
value satisfies Scalar;
```

### saturating multiply uses Multiply

> `*|` uses the same receiver contract as `*`.

```ds
struct Scalar { value: int }

extension of Scalar implements Multiply<Scalar> {
    multiply(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const value = left *| right;
value satisfies Scalar;
```
