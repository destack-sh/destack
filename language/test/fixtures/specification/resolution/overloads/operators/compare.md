# Comparison Operator Overloading

Tests for comparison operator overloading via interface implementations.

## Equality and ordering

### equality and comparison operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Measure { value: int }

extension of Measure implements Equal<Measure>, Compare<Measure> {
    equal(other: Measure): boolean { return true }
    compare(other: Measure): Ordering { return Ordering.Equal }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

const isEqual = left == right;
isEqual satisfies boolean;

const isNotEqual = left != right;
isNotEqual satisfies boolean;

const isLess = left < right;
isLess satisfies boolean;

const isLessEqual = left <= right;
isLessEqual satisfies boolean;

const isGreater = left > right;
isGreater satisfies boolean;

const isGreaterEqual = left >= right;
isGreaterEqual satisfies boolean;
```

### strict equality rejects struct values

> Strict equality is only defined for identity types.

```ds
struct Point { x: int }

declare function getPoint(): Point;

const left = getPoint();
const right = getPoint();

left === right;
left !== right;
```

- contains: strict equality

### comparison operators reject unavailable compare contracts

> Ordering operators require a Compare implementation.

```ds
struct Measure { value: int }

extension of Measure implements Equal<Measure> {
    equal(other: Measure): boolean { return true }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

const isLess = left < right;
isLess satisfies boolean;
```

- type OtherMeasure is not assignable to type Measure

### equality operators reject unavailable equal contracts

> Equality operators require an Equal implementation.

```ds
struct Measure { value: int }

extension of Measure implements Compare<Measure> {
    compare(other: Measure): Ordering { return Ordering.Equal }
}

declare function getMeasure(): Measure;

const left = getMeasure();
const right = getMeasure();

const isEqual = left == right;
isEqual satisfies boolean;
```

- contains: no matching overload

### strict equality accepts primitive operands

> Strict equality remains valid on primitive identity types.

```ds
const same = 1 === 1;
same satisfies boolean;

const different = 1 !== 2;
different satisfies boolean;
```

### comparison contracts require rhs compatibility

> Comparison operators should reject incompatible right operand types even when compare is implemented.

```ds
struct Measure { value: int }
struct OtherMeasure { value: int }

extension of Measure implements Compare<Measure> {
    compare(other: Measure): Ordering { return Ordering.Equal }
}

declare function getMeasure(): Measure;
declare function getOtherMeasure(): OtherMeasure;

const left = getMeasure();
const right = getOtherMeasure();

const isLess = left < right;
isLess satisfies boolean;
```

- type OtherMeasure is not assignable to type Measure