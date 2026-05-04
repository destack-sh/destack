# Bitwise Xor

`^` uses numeric rules for builtin numbers and `Xor` for receiver overloads.

## numbers

### bitwise xor yields the left numeric type

> `^` preserves the left numeric type.

```ds
const value = 5 ^ 3;
value satisfies int32;
```

## overloads

### bitwise xor dispatches to Xor

> `^` dispatches to `Xor` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements Xor<Bits> {
    xor(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left ^ right;
value satisfies Bits;
```
