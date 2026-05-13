# Range Values

Range values use tight operator spacing.

## bounds

### value range bounds

Range operators are attached to their bounds.

```ds
const halfOpen = 1 .. 10
const inclusive = 1 ..= 10
const from = 1 ..
const to = .. 10
const through = ..= 10
const full = ..
```

```ds expected
const halfOpen = 1..10;
const inclusive = 1..=10;
const from = 1..;
const to = ..10;
const through = ..=10;
const full = ..;
```

## endpoints

### arithmetic endpoints

Arithmetic endpoints stay inside the range.

```ds
const window = (start + 1) .. (end * 2)
const nested = (1 .. 4) + count
const negative = -3 .. 3
```

```ds expected
const window = start + 1..end * 2;
const nested = (1..4) + count;
const negative = -3..3;
```
