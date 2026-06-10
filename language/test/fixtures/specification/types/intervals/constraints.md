# Interval Constraints

Interval types can constrain static parameters.

## parameters

### interval constraints accept in-bounds static arguments

An interval bound on a `comptime` parameter checks at instantiation.

```ds
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

let buffer: InlineBuffer<uint8, 64>;
buffer satisfies InlineBuffer<uint8, 64>;
```

### interval constraints reject out-of-bounds static arguments

Out-of-range instantiations fail.

```ds
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

let buffer: InlineBuffer<uint8, 4097>;
```

- contains: not assignable
