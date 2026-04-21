# Unary Operator Overloading

Tests for unary operator overloading via interface implementations.

## Unary operators

### unary operators dispatch

> Operators dispatch to interface methods when the receiver implements them.

```ds
struct Signed { value: int }

extension of Signed implements Negate, Plus, Not {
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

extension of Pointer implements Deref<int> {
    deref(): int { return this.value }
}

declare function getPointer(): Pointer;

const pointer = getPointer();
const derefValue = *pointer;
derefValue satisfies int;
```

### unary operators reject unavailable unary contracts

> Unary operators require the corresponding unary contract implementation.

```ds
struct Signed { value: int }

extension of Signed implements Negate {
    negate(): Signed { return this }
}

declare function getSigned(): Signed;

const value = getSigned();
const positive = +value;
positive satisfies Signed;
```

- no matching overload for type Signed

### dereference rejects unavailable deref contracts

> Dereference requires a matching Deref contract implementation.

```ds
struct Pointer { value: int }

declare function getPointer(): Pointer;

const pointer = getPointer();
const derefValue = *pointer;
derefValue satisfies int;
```

- no matching overload for type Signed

### unary dispatch preserves receiver-specific implementations

> Unary operator dispatch uses the receiver implementation and preserves result type.

```ds
struct Signed { value: int }
struct Unsigned { value: int }

extension of Signed implements Negate {
    negate(): Signed { return this }
}

extension of Unsigned implements Plus {
    plus(): Unsigned { return this }
}

declare function getSigned(): Signed;
declare function getUnsigned(): Unsigned;

const signedValue = -getSigned();
signedValue satisfies Signed;

const unsignedValue = +getUnsigned();
unsignedValue satisfies Unsigned;
```

### unary operators reject receiver-only coverage for missing contracts

> Unary operator dispatch should reject operators whose contracts are not implemented on the receiver.

```ds
struct Signed { value: int }

extension of Signed implements Plus {
    plus(): Signed { return this }
}

declare function getSigned(): Signed;

const value = getSigned();
const negated = -value;
```

- no matching overload for type Signed