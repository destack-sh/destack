# Primitive aliases

## tests

### int uses the default signed integer width

> int uses the default signed integer width.

```ds
declare const value: int32;
let alias: int = value;
let back: int32 = alias;
alias satisfies int32;
```

### uint uses the default unsigned integer width

> uint uses the default unsigned integer width.

```ds
declare const value: uint32;
let alias: uint = value;
let back: uint32 = alias;
alias satisfies uint32;
```

### float uses the default float width

> float uses the default float width.

```ds
declare const value: float64;
let alias: float = value;
let back: float64 = alias;
alias satisfies float64;
```

### number accepts precise numerics

> number accepts precise numeric types but does not narrow back automatically.

```ds
declare const precise: float64;
declare const value: number;
let widen: number = precise;
let narrow: float64 = value;
```

- contains: not assignable

### int alias rejects unsigned values

> `int` aliases signed default width and rejects unsigned assignments.

```ds
declare const value: uint32;
let alias: int = value;
```

- contains: not assignable

### float alias rejects integer-narrow expectations

> `float` aliases default float width and does not narrow to integer aliases.

```ds
declare const value: float;
let alias: int = value;
```

- contains: not assignable
