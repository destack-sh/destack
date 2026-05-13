# Interval Scalars

Interval types apply to integers, `bigint`, and `char`.
Floats, strings, and user-defined ordering do not form intervals.

## chars

### char intervals use Unicode scalar order

```ds
type LowerAscii = 'a'..='z';

let value: LowerAscii = 'm';
value satisfies LowerAscii;
value satisfies char;
```

### char intervals reject values outside their bounds

```ds
type LowerAscii = 'a'..='z';

let value: LowerAscii = 'A';
```

- contains: not assignable

## bigint

### bigint intervals use integer order

```ds
type BigId = 1n..=9007199254740993n;

let value: BigId = 9007199254740993n;
value satisfies BigId;
value satisfies bigint;
```

## unsupported

### float intervals are rejected because floats are partially ordered

```ds
type UnitFloat = 0.0..=1.0;
```

- contains: interval type

### string intervals are rejected because string ordering is not primitive

```ds
type Lower = "a"..="z";
```

- contains: interval type
