# BigInt Literals

Bigint literal type inference and checking.

## bigint

### bigint literal

> Bigint literals can be assigned to bigint type.


```ds
const x: bigint = 42n;
```

## BigInt members

### bigint toString resolves

> Bigint literals expose BigInt standard members.


```ds
const value = 42n;
const text = value.toString();
text satisfies string;
```

### bigint literal is not assignable to number

> Bigint values are not assignable to number.

```ds
const value: number = 42n;
```

- contains: not assignable

### bigint literals participate in bigint arithmetic

> Bigint arithmetic preserves bigint results.

```ds
const value = 40n + 2n;
value satisfies bigint;
```

### bigint arithmetic rejects number operands

> Bigint arithmetic rejects mixed bigint and number operands.

```ds
const value = 40n + 2;
value satisfies bigint;
```

- contains: not assignable
