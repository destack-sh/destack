# Assignment Expressions

Tests for assignment operator spacing and chaining.

## Basic Assignments

### simple assignment

Simple assignments keep spaces around the operator.

```ds
value = 1
```

```ds expected
value = 1;
```

### chained assignment

Chained assignments stay right-associative.

```ds
first = second = third
```

```ds expected
first = second = third;
```

## Compound Assignments

### additive assignment

Compound assignments keep spaces around the operator.

```ds
count += 1
```

```ds expected
count += 1;
```

### logical assignments

Logical assignment operators format with spaces.

```ds
value ||= fallback
```

```ds expected
value ||= fallback;
```

### logical and assignment

Logical and assignment operators format with spaces.

```ds
value &&= compute()
```

```ds expected
value &&= compute();
```

### bitwise assignment

Bitwise assignments keep spaces around the operator.

```ds
flags |= mask
```

```ds expected
flags |= mask;
```

### shift assignment

Shift assignments keep spaces around the operator.

```ds
flags <<= 1
```

```ds expected
flags <<= 1;
```
