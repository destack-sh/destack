# Primitive aliases

## aliases

### int uses the default signed integer width

> Int uses the default signed integer width.

```ds
declare const value: int32;
let alias: int = value;
let back: int32 = alias;
alias satisfies int32;
```

### uint uses the default unsigned integer width

> Uint uses the default unsigned integer width.

```ds
declare const value: uint32;
let alias: uint = value;
let back: uint32 = alias;
alias satisfies uint32;
```

### float uses the default float width

> Float uses the default float width.

```ds
declare const value: float64;
let alias: float = value;
let back: float64 = alias;
alias satisfies float64;
```

### number accepts precise numerics

> Number accepts precise numeric types but does not narrow back automatically.

```ds
declare const precise: float64;
declare const value: number;
let widen: number = precise;
let narrow: float64 = value;
```

- type float64 is not assignable to type int32

### int alias rejects unsigned values

> `int` aliases signed default width and rejects unsigned assignments.

```ds
declare const value: uint32;
let alias: int = value;
```

- type float64 is not assignable to type int32

### float alias rejects integer-narrow expectations

> `float` aliases default float width and does not narrow to integer aliases.

```ds
declare const value: float;
let alias: int = value;
```

- type float64 is not assignable to type int32
