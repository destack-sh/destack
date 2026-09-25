# Binary Expressions

Binary fixtures cover operator spacing, precedence, flattening, and line breaking.

## Logical Chains

### and chain breaks with leading operators

Logical chains break with leading operators when they exceed line width.

```tspp line-width=12
a && b && c && d
```

```tspp expected
a
    && b
    && c
    && d;
```

### or chain breaks with leading operators

Logical OR chains break with leading operators.

```tspp line-width=12
a || b || c || d
```

```tspp expected
a
    || b
    || c
    || d;
```

### nullish chain breaks with leading operators

Nullish coalescing chains break with leading operators.

```tspp line-width=12
a ?? b ?? c ?? d
```

```tspp expected
a
    ?? b
    ?? c
    ?? d;
```

### nullish coalescing

Nullish coalescing stays compact when it fits.

```tspp
const result = primary ?? secondary ?? fallback
```

```tspp expected
const result = primary ?? secondary ?? fallback;
```

## Mixed Operators

### logical with grouped arithmetic

Arithmetic groups stay inline inside logical expressions.

```tspp line-width=40
(a + b * c) && (d - e / f)
```

```tspp expected
a + b * c && d - e / f;
```

### comparison chain with logical operator

Comparison expressions align with logical operators on breaks.

```tspp line-width=20
a <= b && c >= d && e <= f
```

```tspp expected
a <= b
    && c >= d
    && e <= f;
```

### in and instanceof spacing

Container operators keep spaces around them.

```tspp
value in container && value instanceof Type
```

```tspp expected
value in container && value instanceof Type;
```

### additive and multiplicative precedence

Multiplication binds tighter than addition without extra parentheses.

```tspp
a + b * c - d / e
```

```tspp expected
a + b * c - d / e;
```

### bitwise and logical mix

Bitwise operators remain inline when mixed with logical operators.

```tspp
flags & mask && ready
```

```tspp expected
flags & mask && ready;
```

## Bitwise Operators

### bitwise precedence stays explicit

Mixed bitwise operators preserve precedence with explicit grouping.

```tspp
flags & mask | other
```

```tspp expected
(flags & mask) | other;
```

## Comments in Chains

### comments stay with operands

Comments stay with the following operand on breaks.

```tspp line-width=20
a && /* keep */ b && /* keep */ c
```

```tspp expected
a
    && /* keep */ b
    && /* keep */ c;
```
