# Shift

`<<`, `>>`, and `>>>` use numeric rules for builtin numbers and shift contracts for receiver overloads.

## numbers

### shift left yields the left numeric type

> `<<` preserves the left numeric type.

```ds
const value = 1 << 2;
value satisfies int32;
```

### shift right yields the left numeric type

> `>>` preserves the left numeric type.

```ds
const value = 8 >> 1;
value satisfies int32;
```

### unsigned shift right yields the left numeric type

> `>>>` preserves the left numeric type.

```ds
const value = 8 >>> 1;
value satisfies int32;
```

## overloads

### shift left dispatches to ShiftLeft

> `<<` dispatches to `ShiftLeft` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements ShiftLeft<Bits> {
    type Output = Bits;

    shiftLeft(other: Bits): this.Output { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left << right;
value satisfies Bits;
```

### shift right dispatches to ShiftRight

> `>>` dispatches to `ShiftRight` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements ShiftRight<Bits> {
    type Output = Bits;

    shiftRight(other: Bits): this.Output { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left >> right;
value satisfies Bits;
```

### unsigned shift right dispatches to ShiftRightUnsigned

> `>>>` dispatches to `ShiftRightUnsigned` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements ShiftRightUnsigned<Bits> {
    type Output = Bits;

    shiftRightUnsigned(other: Bits): this.Output { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left >>> right;
value satisfies Bits;
```

### shift requires the right operand type

> `ShiftLeft<T>` only accepts right operands assignable to `T`.

```ds
struct Bits { value: int }
struct OtherBits { value: int }

extension of Bits implements ShiftLeft<Bits> {
    type Output = Bits;

    shiftLeft(other: Bits): this.Output { return this }
}

declare function getBits(): Bits;
declare function getOtherBits(): OtherBits;

const left = getBits();
const right = getOtherBits();

left << right;
```

- contains: not assignable
