# Bitwise Or

`|` uses numeric rules for builtin numbers and `Or` for receiver overloads.

## numbers

### bitwise or yields the left numeric type

> `|` preserves the left numeric type.

```ds
const value = 5 | 3;
value satisfies int32;
```

## builtins

### sets use bitwise or for union

> Sets support `|` union through the builtin `Or` implementation.

```ds libs=es2015
declare const left: Set<int32>;
declare const right: Set<int32>;

const combined = left | right;
combined satisfies Set<int32>;
```

### maps use bitwise or for merge

> Maps support `|` merge through the builtin `Or` implementation.

```ds libs=es2015
declare const left: Map<string, int32>;
declare const right: Map<string, int32>;

const combined = left | right;
combined satisfies Map<string, int32>;
```

## overloads

### bitwise or dispatches to Or

> `|` dispatches to `Or` on the receiver.

```ds
struct Bits { value: int }

extension of Bits implements Or<Bits> {
    or(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

const value = left | right;
value satisfies Bits;
```

### bitwise or requires Or

> `|` requires a matching `Or` implementation.

```ds
struct Bits { value: int }

extension of Bits implements And<Bits> {
    and(other: Bits): Bits { return this }
}

declare function getBits(): Bits;

const left = getBits();
const right = getBits();

left | right;
```

- contains: no matching overload
