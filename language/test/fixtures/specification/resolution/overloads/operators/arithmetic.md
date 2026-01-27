# Arithmetic Operator Overloading

Tests for arithmetic operator overloading via interface implementations.

## Addition and subtraction

### arithmetic operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Scalar { value: int }

extension for Scalar implements Add<Scalar>, Subtract<Scalar>, Multiply<Scalar>, Divide<Scalar>, Remainder<Scalar>, Power<Scalar> {
    add(other: Scalar): Scalar { return this }
    subtract(other: Scalar): Scalar { return this }
    multiply(other: Scalar): Scalar { return this }
    divide(other: Scalar): Scalar { return this }
    remainder(other: Scalar): Scalar { return this }
    power(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const add = left + right;
add satisfies Scalar;

const addWrapping = left +% right;
addWrapping satisfies Scalar;

const addSaturating = left +| right;
addSaturating satisfies Scalar;

const sub = left - right;
sub satisfies Scalar;

const subWrapping = left -% right;
subWrapping satisfies Scalar;

const subSaturating = left -| right;
subSaturating satisfies Scalar;

const mul = left * right;
mul satisfies Scalar;

const mulWrapping = left *% right;
mulWrapping satisfies Scalar;

const mulSaturating = left *| right;
mulSaturating satisfies Scalar;

const div = left / right;
div satisfies Scalar;

const remainder = left % right;
remainder satisfies Scalar;

const pow = left ** right;
pow satisfies Scalar;

const powWrapping = left **% right;
powWrapping satisfies Scalar;

const powSaturating = left **| right;
powSaturating satisfies Scalar;
```
