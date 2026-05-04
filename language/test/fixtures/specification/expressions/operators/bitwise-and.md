# Bitwise And

`&` uses numeric rules for builtin numbers and `And` for receiver overloads.

## numbers

### bitwise and yields the left numeric type

> `&` preserves the left numeric type.

```ds
const value = 5 & 3;
value satisfies int32;
```

## overloads

### bitwise and dispatches to And

> `&` dispatches to `And` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements And<Bits> {
    and(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left & right;
value satisfies Bits;
```

### bitwise and requires And

> `&` requires a matching `And` implementation.

```ds
struct Bits { value: int }

extension of Bits implements ShiftLeft<Bits> {
    shiftLeft(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

left & right;
```

- contains: no matching overload

### bitwise and requires the right operand type

> `And<R>` only accepts right operands assignable to `R`.

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

left & right;
```

- contains: no matching overload
