# Variable Width Integers

## ranges

### integer literals satisfy fitting widths

```ds
7 satisfies uint3;
7 satisfies int4;
```

### integer literals reject narrow widths

```ds
7 satisfies uint2;
```

- contains: not assignable

### intN accepts in-range literals

```ds
let small: int3 = 3;
```

### intN rejects out-of-range literals

```ds
let tooLarge: int3 = 4;
```

- contains: not assignable

### uintN accepts in-range literals

```ds
let small: uint3 = 7;
```

### uintN rejects out-of-range literals

```ds
let tooLarge: uint3 = 8;
```

- contains: not assignable

### pointer-sized integers accept literals

```ds
let signed: isize = 0;
let unsigned: usize = 1;
```
