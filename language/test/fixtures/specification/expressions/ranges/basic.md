# Range Basics

## exclusive ranges

### exclusive range expression is allowed

> Exclusive ranges are valid expressions.

```ds
let range = 0..10;
range;
```

## inclusive ranges

### inclusive range expression is allowed

> Inclusive ranges are valid expressions.

```ds
let range = 0..=10;
range;
```

## variable ranges

### range expressions accept variable bounds

> Range bounds can be arbitrary expressions.

```ds
let start = 1;
let end = 5;
let range = start..end;
range;
```

## iteration

### range expressions can be iterated

> Range expressions can appear in for-of loops.

```ds
for (const i of 0..10) {
    i;
}
```
