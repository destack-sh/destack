# Unary Operator Overloading

Tests for unary operator overloading via interface implementations.

## Unary operators

### unary operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Signed { value: int }

extension for Signed implements Negate, Plus, Not {
    negate(): Signed { return this }
    plus(): Signed { return this }
    not(): Signed { return this }
}

declare function getSigned(): Signed;

const value = getSigned();

const negated = -value;
negated satisfies Signed;

const positive = +value;
positive satisfies Signed;

const inverted = ~value;
inverted satisfies Signed;
```

### dereference operator dispatches

> The dereference operator uses the Deref interface when implemented.

```ds
struct Pointer { value: int }

extension for Pointer implements Deref<int> {
    deref(): int { return this.value }
}

declare function getPointer(): Pointer;

const pointer = getPointer();
const derefValue = *pointer;
derefValue satisfies int;
```
