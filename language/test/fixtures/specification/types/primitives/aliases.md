# Numeric Aliases

The numeric aliases name the same types as their spelled-out forms.

## integer aliases

### int uses int64

`int` is the `int64` alias.

```ds
declare const value: int64;
let alias: int = value;
let back: int64 = alias;
alias satisfies int64;
```

### uint uses uint64

`uint` is the `uint64` alias.

```ds
declare const value: uint64;
let alias: uint = value;
let back: uint64 = alias;
alias satisfies uint64;
```

### int rejects uint64

Signed and unsigned aliases remain distinct.

```ds
declare const value: uint64;
let alias: int = value;
```

- contains: not assignable

### uint rejects int64

Signed and unsigned aliases remain distinct.

```ds
declare const value: int64;
let alias: uint = value;
```

- contains: not assignable

## float aliases

### float uses float64

`float` is the `float64` alias.

```ds
declare const value: float64;
let alias: float = value;
let back: float64 = alias;
alias satisfies float64;
```

### number uses float

`number` is TypeScript's spelling for `float`.

```ds
declare const value: float;
let alias: number = value;
let back: float = alias;
alias satisfies float;
alias satisfies float64;
```

### number accepts precise numerics

`number` accepts precise numeric types but does not narrow back automatically.

```ds
declare const precise: int32;
declare const value: number;
let widen: number = precise;
let narrow: int32 = value;
```

- contains: not assignable
