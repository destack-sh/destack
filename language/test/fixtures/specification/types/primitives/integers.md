# Arbitrary Width Integers

## ranges

### intN accepts in-range literals

> Arbitrary width signed integers accept literals in range.

```ds
let small: int3 = 3;
```

### intN rejects out-of-range literals

> Arbitrary width signed integers reject literals out of range.

```ds
let tooLarge: int3 = 4;
```

- contains: not assignable

### uintN accepts in-range literals

> Arbitrary width unsigned integers accept literals in range.

```ds
let small: uint3 = 7;
```

### uintN rejects out-of-range literals

> Arbitrary width unsigned integers reject literals out of range.

```ds
let tooLarge: uint3 = 8;
```

- contains: not assignable

### pointer-sized integers accept literals

> Pointer-sized integer types accept integer literals.

```ds
let signed: isize = 0;
let unsigned: usize = 1;
```
