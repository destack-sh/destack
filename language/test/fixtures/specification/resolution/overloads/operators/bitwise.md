# Bitwise and Shift Operator Overloading

Tests for bitwise and shift operator overloading via interface implementations.

## Bitwise and shift operators

### bitwise and shift operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Bits { value: int }

extension of Bits implements ShiftLeft<Bits>, ShiftRight<Bits>, ShiftRightUnsigned<Bits>, And<Bits>, Or<Bits>, Xor<Bits> {
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

### bitwise and shift operators reject unavailable methods

> Bitwise and shift operators require their corresponding implemented operator contracts.

```ds
struct Bits { value: int }

extension of Bits implements ShiftLeft<Bits>, And<Bits> {
    shiftLeft(other: Bits): Bits { return this }
    and(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const shiftLeft = left << right;
shiftLeft satisfies Bits;

const shiftUnsigned = left >>> right;
shiftUnsigned satisfies Bits;
```

- contains: no matching overload

### bitwise and shift operators require rhs compatibility

> Bitwise and shift operator dispatch requires a compatible right operand type.

```ds
struct Bits { value: int }
struct OtherBits { value: int }

extension of Bits implements And<Bits> {
    and(other: Bits): Bits { return this }
}

declare function getBits(): Bits;
declare function getOtherBits(): OtherBits;

const left = getBits();
const right = getOtherBits();

const andValue = left & right;
andValue satisfies Bits;
```

- contains: no matching overload

### bitwise and shift operators do not use rhs-only implementations

> Receiver-based operator dispatch does not accept rhs-only bitwise implementations.

```ds
struct Bits { value: int }
struct OtherBits { value: int }

extension of OtherBits implements And<Bits> {
    and(other: Bits): OtherBits { return this }
}

declare function getBits(): Bits;
declare function getOtherBits(): OtherBits;

const left = getBits();
const right = getOtherBits();

const andValue = left & right;
andValue satisfies OtherBits;
```

- contains: no matching overload

### saturating shift operators require shift contracts

> Saturating shift operators use the same shift contract requirements.

```ds
struct Bits { value: int }

extension of Bits implements ShiftLeft<Bits> {
    shiftLeft(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const shifted = left <<| right;
shifted satisfies Bits;
```

### bitwise shift operators require rhs compatibility

> Shift operators should reject incompatible right operand types.

```ds
struct Bits { value: int }
struct OtherBits { value: int }

extension of Bits implements ShiftLeft<Bits> {
    shiftLeft(other: Bits): Bits { return this }
}

declare function getBits(): Bits;
declare function getOtherBits(): OtherBits;

const left = getBits();
const right = getOtherBits();

const shifted = left << right;
shifted satisfies Bits;
```

- type OtherBits is not assignable to type Bits