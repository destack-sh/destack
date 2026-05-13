# Interval Bounds

Interval types are static subsets of integers, `bigint`, or `char`.

## forms

### half-open intervals include the start and exclude the end

```ds
type Window = 0..10;

let value: Window = 5;
value satisfies Window;
value satisfies int;
```

### inclusive intervals include both bounds

```ds
type Window = 0..=10;

let value: Window = 10;
value satisfies Window;
value satisfies int;
```

### one-sided intervals constrain one side

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

```ds
type Window = 0..10;

let value: Window = 10;
```

- contains: not assignable

### inclusive intervals accept their end bound

```ds
type Window = 0..=10;

let value: Window = 10;
value satisfies Window;
```

### interval types compose with unions

```ds
type Digit = 0..=9;
type HexLetter = "a" | "b" | "c" | "d" | "e" | "f";
type Hex = Digit | HexLetter;

let value: Hex = 9;
value satisfies Hex;
```

### interval aliases preserve precision

```ds
type Byte = 0..=255;
type Port = 1..=65535;

let byte: Byte = 255;
let port: Port = 443;

byte satisfies Byte;
port satisfies Port;
```

### interval assignments accept refined values

```ds
type Digit = 0..=9;

declare const digit: Digit;

let value: Digit = digit;
value satisfies Digit;
```

### interval assignments reject unrefined values

```ds
type Digit = 0..=9;

declare const value: int;

let digit: Digit = value;
```

- contains: not assignable

### interval assignments do not prove arithmetic

```ds
type Digit = 0..=9;

declare const digit: Digit;

let next: Digit = digit + 1;
```

- contains: not assignable

### interval variables reject unchecked mutation

```ds
type Digit = 0..=9;

let digit: Digit = 5;
digit += 1;
```

- contains: not assignable

### full range spelling is not an interval

```ds
type Everything = ..;
```

- contains: interval type
