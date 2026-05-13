# Interval Constraints

Interval types can constrain static parameters.

## parameters

### interval constraints accept in-bounds static arguments

```ds
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

let buffer: InlineBuffer<uint8, 64>;
buffer satisfies InlineBuffer<uint8, 64>;
```

### interval constraints reject out-of-bounds static arguments

```ds
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

let buffer: InlineBuffer<uint8, 4097>;
```

- contains: not assignable
