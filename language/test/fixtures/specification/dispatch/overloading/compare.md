# Comparison Operator Overloading

Tests for comparison operator overloading via interface implementations.

## Equality and ordering

> TODO #Broken: comparison operators should lower compare to Ordering checks that return boolean
 (probably should do this in desugar during Bind?)

### _equality and comparison operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Measure { value: int }

extension for Measure implements Equal<Measure>, Compare<Measure> {
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
