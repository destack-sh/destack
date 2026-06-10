# Interval Bounds

Interval types are static subsets of integers, `bigint`, or `char`.

## forms

### half-open intervals include the start and exclude the end

`0..10` admits `0` through `9`.

```ds
type Window = 0..10;

let value: Window = 5;
value satisfies Window;
value satisfies int;
```

### inclusive intervals include both bounds

`..=` admits the end itself.

```ds
type Window = 0..=10;

let value: Window = 10;
value satisfies Window;
value satisfies int;
```

### one-sided intervals constrain one side

The open side stays unbounded within the scalar.

```ds
type From = 10..;
type To = ..10;
type Through = ..=10;

let from: From = 11;
let to: To = 9;
let through: Through = 10;
```

## assignability

### interval types reject values outside their bounds

The excluded end is out.

```ds
type Window = 0..10;

let value: Window = 10;
```

- contains: not assignable

### inclusive intervals accept their end bound

`..=` keeps the boundary value legal.

```ds
type Window = 0..=10;

let value: Window = 10;
value satisfies Window;
```

### interval types compose with unions

Intervals are just literal unions, so they union freely.

```ds
type Digit = 0..=9;
type HexLetter = "a" | "b" | "c" | "d" | "e" | "f";
type Hex = Digit | HexLetter;

let value: Hex = 9;
value satisfies Hex;
```

### interval aliases preserve precision

Aliases keep the exact bounds.

```ds
type Byte = 0..=255;
type Port = 1..=65535;

let byte: Byte = 255;
let port: Port = 443;

byte satisfies Byte;
port satisfies Port;
```

### interval assignments accept refined values

A value already typed at the interval flows through.

```ds
type Digit = 0..=9;

declare const digit: Digit;

let value: Digit = digit;
value satisfies Digit;
```

### interval assignments reject unrefined values

An unproven scalar does not narrow itself.

```ds
type Digit = 0..=9;

declare const value: int;

let digit: Digit = value;
```

- contains: not assignable

### interval assignments do not prove arithmetic

The compiler does not do interval arithmetic.

```ds
type Digit = 0..=9;

declare const digit: Digit;

let next: Digit = digit + 1;
```

- contains: not assignable

### interval variables reject unchecked mutation

Compound assignment is arithmetic too.

```ds
type Digit = 0..=9;

let digit: Digit = 5;
digit += 1;
```

- contains: not assignable

### full range spelling is not an interval

`..` has no bounds to refine.

```ds
type Everything = ..;
```

- contains: interval type
