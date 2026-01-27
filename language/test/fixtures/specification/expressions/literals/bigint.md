# BigInt Literals

Tests for bigint literal type inference and checking.

## Basic BigInt

### bigint literal

> Bigint literals can be assigned to bigint type.


```ds libs=es2020
const x: bigint = 42n;
```

## BigInt Members

### bigint toString resolves

> Bigint literals expose BigInt prototype members.


```ds libs=es2020
const value = 42n;
const text = value.toString();
text satisfies string;
```
