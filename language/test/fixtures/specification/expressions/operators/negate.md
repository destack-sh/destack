# Negate

Unary `-` uses numeric rules for builtin numbers and `Negate` for receiver overloads.

## overloads

### negate dispatches to Negate

> Unary `-` dispatches to `Negate` on the receiver.

```ds
struct Signed { value: int }

extension of Signed implements Negate {
    negate(): Signed { return this }
}

declare function getSigned(): Signed;

const value = -getSigned();
value satisfies Signed;
```

### negate requires Negate

> Unary `-` requires a matching `Negate` implementation.

```ds
struct Signed { value: int }

extension of Signed implements Plus {
    plus(): Signed { return this }
}

declare function getSigned(): Signed;

const value = -getSigned();
value satisfies Signed;
```

- contains: no matching overload for type Signed
