# Binary Expressions

Binary fixtures cover operator spacing, precedence, flattening, and line breaking.

## Logical Chains

### and chain breaks with leading operators

Logical chains break with leading operators when they exceed line width.

```ds line-width=12
a && b && c && d
```

```ds expected
a &&
    b &&
    c &&
    d;
```

### or chain breaks with leading operators

Logical OR chains break with leading operators.

```ds line-width=12
a || b || c || d
```

```ds expected
a ||
    b ||
    c ||
    d;
```

### nullish chain breaks with leading operators

Nullish coalescing chains break with leading operators.

```ds line-width=12
a ?? b ?? c ?? d
```

```ds expected
a ??
    b ??
    c ??
    d;
```

## Mixed Operators

### logical with grouped arithmetic

Arithmetic groups stay inline inside logical expressions.

```ds line-width=40
(a + b * c) && (d - e / f)
```

```ds expected
a + b * c && d - e / f;
```

### comparison chain with logical operator

Comparison expressions align with logical operators on breaks.

```ds line-width=20
a <= b && c >= d && e <= f
```

```ds expected
a <= b &&
    c >= d &&
    e <= f;
```

### in and instanceof spacing

Container operators keep spaces around them.

```ds
value in container && value instanceof Type
```

```ds expected
value in container && value instanceof Type;
```

### additive and multiplicative precedence

Multiplication binds tighter than addition without extra parentheses.

```ds
a + b * c - d / e
```

```ds expected
a + b * c - d / e;
```

### bitwise and logical mix

Bitwise operators remain inline when mixed with logical operators.

```ds
flags & mask && ready
```

```ds expected
flags & mask && ready;
```

## Bitwise Operators

### bitwise precedence stays explicit

Mixed bitwise operators preserve precedence with explicit grouping.

```ds
flags & mask | other
```

```ds expected
(flags & mask) | other;
```

## Destack Arithmetic Variants

### wrapping operators stay spaced

Wrapping operators keep spaces around them.

```ds
total +% increment
```

```ds expected
total +% increment;
```

```ds
product *% multiplier
```

```ds expected
product *% multiplier;
```

### saturating operators stay spaced

Saturating operators keep spaces around them.

```ds
total +| increment
```

```ds expected
total +| increment;
```

```ds
product *| multiplier
```

```ds expected
product *| multiplier;
```

## Comments in Chains

### comments stay with operands

Comments stay with the following operand on breaks.

```ds line-width=20
a && /* keep */ b && /* keep */ c
```

```ds expected
a &&
    /* keep */ b &&
    /* keep */ c;
```
