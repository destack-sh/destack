# Arithmetic Operator Overloading

Tests for arithmetic operator overloading via interface implementations.

## Addition and subtraction

### arithmetic operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Scalar { value: int }

extension of Scalar implements Add<Scalar>, Subtract<Scalar>, Multiply<Scalar>, Divide<Scalar>, Remainder<Scalar>, Power<Scalar> {
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

### arithmetic operators reject unavailable methods

> Arithmetic operators require the corresponding implemented operator contract.

```ds
struct Scalar { value: int }

extension of Scalar implements Add<Scalar> {
    add(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const add = left + right;
add satisfies Scalar;

const subtract = left - right;
subtract satisfies Scalar;
```

- contains: no matching overload

### arithmetic operators require rhs compatibility

> Arithmetic operator dispatch requires a compatible right operand type.

```ds
struct Scalar { value: int }
struct Other { value: int }

extension of Scalar implements Add<Scalar> {
    add(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;
declare function getOther(): Other;

const left = getScalar();
const right = getOther();

const add = left + right;
add satisfies Scalar;
```

- contains: no matching overload

### arithmetic operators do not use rhs-only implementations

> Receiver-based operator dispatch does not accept rhs-only overload implementations.

```ds
struct Scalar { value: int }
struct Other { value: int }

extension of Other implements Add<Scalar> {
    add(other: Scalar): Other { return this }
}

declare function getScalar(): Scalar;
declare function getOther(): Other;

const left = getScalar();
const right = getOther();

const add = left + right;
add satisfies Other;
```

- contains: no matching overload

### arithmetic wrapping operators require the same receiver contracts

> Wrapping arithmetic operators use the same receiver-based contract requirements.

```ds
struct Scalar { value: int }

extension of Scalar implements Add<Scalar> {
    add(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;

const left = getScalar();
const right = getScalar();

const wrapped = left +% right;
wrapped satisfies Scalar;
```

### arithmetic wrapping operators require rhs compatibility

> Wrapping arithmetic operators still require compatible right operand types.

```ds
struct Scalar { value: int }
struct Other { value: int }

extension of Scalar implements Add<Scalar> {
    add(other: Scalar): Scalar { return this }
}

declare function getScalar(): Scalar;
declare function getOther(): Other;

const left = getScalar();
const right = getOther();

const wrapped = left +% right;
wrapped satisfies Scalar;
```

- type Other is not assignable to type Scalar