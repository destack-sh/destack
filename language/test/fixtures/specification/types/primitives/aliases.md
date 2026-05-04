# Primitive Aliases

## aliases

### int uses the default signed integer width

> `int` uses the default signed integer width.

```ds
declare const value: int32;
let alias: int = value;
let back: int32 = alias;
alias satisfies int32;
```

### uint uses the default unsigned integer width

> `uint` uses the default unsigned integer width.

```ds
declare const value: uint32;
let alias: uint = value;
let back: uint32 = alias;
alias satisfies uint32;
```

### number uses float64

> `number` is the `float64` alias.

```ds
declare const value: float64;
let alias: number = value;
let back: float64 = alias;
alias satisfies float64;
```

### number accepts precise numerics

> Number accepts precise numeric types but does not narrow back automatically.

```ds
declare const precise: int32;
declare const value: number;
let widen: number = precise;
let narrow: int32 = value;
```

- contains: not assignable

### int alias rejects unsigned values

> `int` aliases signed default width and rejects unsigned assignments.

```ds
declare const value: uint32;
let alias: int = value;
```

- contains: not assignable
