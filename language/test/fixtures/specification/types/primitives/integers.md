# Integers

Integer types carry exact widths and ranges.

## ranges

### integer literals satisfy fitting widths

Literals know their bit requirements.

```ds
7 satisfies uint3;
7 satisfies int4;
```

### integer literals reject narrow widths

A literal cannot shrink below its value.

```ds
7 satisfies uint2;
```

- contains: not assignable

### intN accepts in-range literals

Arbitrary-width signed integers hold their range.

```ds
let small: int3 = 3;
```

### intN rejects out-of-range literals

The range is exact.

```ds
let tooLarge: int3 = 4;
```

- contains: not assignable

### uintN accepts in-range literals

Arbitrary-width unsigned integers hold their range.

```ds
let small: uint3 = 7;
```

### uintN rejects out-of-range literals

The range is exact.

```ds
let tooLarge: uint3 = 8;
```

- contains: not assignable

### pointer-sized integers accept literals

`isize` and `usize` follow the target word size.

```ds
let signed: isize = 0;
let unsigned: usize = 1;
```
