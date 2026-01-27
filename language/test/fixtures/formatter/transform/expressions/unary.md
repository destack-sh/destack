# Unary Expressions

Tests for unary operator spacing and grouping.

## Prefix Operators

### logical not

Logical not attaches directly to the operand.

```ds
!isReady
```

```ds expected
!isReady;
```

### numeric negation

Unary minus attaches directly to the operand.

```ds
-total
```

```ds expected
-total;
```

### bitwise not

Bitwise not attaches directly to the operand.

```ds
~mask
```

```ds expected
~mask;
```

## Keyword Operators

### typeof operator

Typeof keeps a space before the operand.

```ds
typeof value
```

```ds expected
typeof value;
```

### delete operator

Delete keeps a space before the operand.

```ds
delete obj.field
```

```ds expected
delete obj.field;
```

### await operator

Await keeps a space before the operand.

```ds
await fetchData()
```

```ds expected
await fetchData();
```
