# Bitwise and Shift Operator Overloading

Tests for bitwise and shift operator overloading via interface implementations.

## Bitwise and shift operators

### bitwise and shift operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Bits { value: int }

extension for Bits implements ShiftLeft<Bits>, ShiftRight<Bits>, ShiftRightUnsigned<Bits>, And<Bits>, Or<Bits>, Xor<Bits> {
    shiftLeft(other: Bits): Bits { return this }
    shiftRight(other: Bits): Bits { return this }
    shiftRightUnsigned(other: Bits): Bits { return this }
    and(other: Bits): Bits { return this }
    or(other: Bits): Bits { return this }
    xor(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const shiftLeft = left << right;
shiftLeft satisfies Bits;

const shiftLeftSaturating = left <<| right;
shiftLeftSaturating satisfies Bits;

const shiftRight = left >> right;
shiftRight satisfies Bits;

const shiftUnsigned = left >>> right;
shiftUnsigned satisfies Bits;

const andValue = left & right;
andValue satisfies Bits;

const orValue = left | right;
orValue satisfies Bits;

const xorValue = left ^ right;
xorValue satisfies Bits;
```
