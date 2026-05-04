# Bitwise Not

`~` uses numeric rules for builtin numbers and `Not` for receiver overloads.

## numbers

### bitwise not yields the operand numeric type

> `~` preserves the operand numeric type.

```ds
const value = ~5;
value satisfies int32;
```

## overloads

### bitwise not dispatches to Not

> `~` dispatches to `Not` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements Not {
    not(): Bits { return this }
}

declare function getBits(): Bits;

const value = ~getBits();
value satisfies Bits;
```

### bitwise not requires Not

> `~` requires a matching `Not` implementation.

```ds
struct Bits { value: int }

extension of Bits implements Plus {
    plus(): Bits { return this }
}

declare function getBits(): Bits;

const value = ~getBits();
value satisfies Bits;
```

- contains: no matching overload for type Bits
