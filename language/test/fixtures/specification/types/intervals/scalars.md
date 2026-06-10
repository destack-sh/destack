# Interval Scalars

Interval types apply to integers, `bigint`, and `char`.
Floats, strings, and user-defined ordering do not form intervals.

## chars

### char intervals use Unicode scalar order

`char` is a bounded set with a total order.

```ds
type LowerAscii = 'a'..='z';

let value: LowerAscii = 'm';
value satisfies LowerAscii;
value satisfies char;
```

### char intervals reject values outside their bounds

The bounds are exact.

```ds
type LowerAscii = 'a'..='z';

let value: LowerAscii = 'A';
```

- contains: not assignable

## bigint

### bigint intervals use integer order

`bigint` intervals work past the float-safe range.

```ds
type BigId = 1n..=9007199254740993n;

let value: BigId = 9007199254740993n;
value satisfies BigId;
value satisfies bigint;
```

## unsupported

### float intervals are rejected because floats are partially ordered

`NaN` breaks the ordering an interval needs.

```ds
type UnitFloat = 0.0..=1.0;
```

- contains: interval type

### string intervals are rejected because string ordering is not primitive

Strings are not a bounded scalar set.

```ds
type Lower = "a"..="z";
```

- contains: interval type
