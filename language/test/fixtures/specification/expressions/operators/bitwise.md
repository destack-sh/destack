# Bitwise

Bitwise operators use numeric rules for builtin numbers and receiver overloads for user types.

## numbers

### bitwise and yields the left numeric type

`&` preserves the left numeric type.

```ds
const value = 5 & 3;
value satisfies int32;
```

### bitwise or yields the left numeric type

`|` preserves the left numeric type.

```ds
const value = 5 | 3;
value satisfies int32;
```

### bitwise xor yields the left numeric type

`^` preserves the left numeric type.

```ds
const value = 5 ^ 3;
value satisfies int32;
```

### bitwise not yields the operand numeric type

`~` preserves the operand numeric type.

```ds
const value = ~5;
value satisfies int32;
```

## builtins

### sets use bitwise or for union

Sets support `|` union through the builtin `Or` implementation.

```ds
declare const left: Set<int32>;
declare const right: Set<int32>;

const combined = left | right;
combined satisfies Set<int32>;
```

### maps use bitwise or for merge

Maps support `|` merge through the builtin `Or` implementation.

```ds
declare const left: Map<string, int32>;
declare const right: Map<string, int32>;

const combined = left | right;
combined satisfies Map<string, int32>;
```

## overloads

### bitwise and dispatches to And

`&` dispatches to `And` on the receiver.

```ds
struct Bits {
    value: int;
}

extension of Bits implements And<Bits> {
    type Output = Bits;

    and(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left & right;
value satisfies Bits;
```

### bitwise or dispatches to Or

`|` dispatches to `Or` on the receiver.

```ds
struct Bits {
    value: int;
}

extension of Bits implements Or<Bits> {
    type Output = Bits;

    or(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left | right;
value satisfies Bits;
```

### bitwise xor dispatches to Xor

`^` dispatches to `Xor` on the receiver.

```ds
struct Bits {
    value: int;
}

extension of Bits implements Xor<Bits> {
    type Output = Bits;

    xor(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left ^ right;
value satisfies Bits;
```

### bitwise not dispatches to Not

`~` dispatches to `Not` on the receiver.

```ds
struct Bits {
    value: int;
}

extension of Bits implements Not {
    type Output = Bits;

    not(): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const value = ~getBits();
value satisfies Bits;
```

### bitwise and requires And

`&` requires a matching `And` implementation.

```ds
struct Bits {
    value: int;
}

extension of Bits implements ShiftLeft<Bits> {
    type Output = Bits;

    shiftLeft(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

left & right;
```

- contains: no matching overload

### bitwise or requires Or

`|` requires a matching `Or` implementation.

```ds
struct Bits {
    value: int;
}

extension of Bits implements And<Bits> {
    type Output = Bits;

    and(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

left | right;
```

- contains: no matching overload

### bitwise xor requires Xor

`^` requires a matching `Xor` implementation.

```ds
struct Bits {
    value: int;
}

extension of Bits implements And<Bits> {
    type Output = Bits;

    and(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

left ^ right;
```

- contains: no matching overload

### bitwise not requires Not

`~` requires a matching `Not` implementation.

```ds
struct Bits {
    value: int;
}

extension of Bits implements Plus {
    type Output = Bits;

    plus(): this.Output {
        return this;
    }
}

declare function getBits(): Bits;

const value = ~getBits();
value satisfies Bits;
```

- contains: no matching overload for type Bits

### bitwise and requires the right operand type

`And<T>` only accepts right operands assignable to `T`.

```ds
struct Bits {
    value: int;
}
struct OtherBits {
    value: int;
}

extension of Bits implements And<Bits> {
    type Output = Bits;

    and(other: Bits): this.Output {
        return this;
    }
}

declare function getBits(): Bits;
declare function getOtherBits(): OtherBits;

const left = getBits();
const right = getOtherBits();

left & right;
```

- contains: no matching overload
