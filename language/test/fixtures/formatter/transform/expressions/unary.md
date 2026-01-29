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

## Destack Reference Operators

### reference operator

References keep the `&` tight to the operand.

```ds
const view = &value
```

```ds expected
const view = &value;
```

### mutable reference operator

Mutable references keep `&mut` tight to the operand.

```ds
const view = &mut value
```

```ds expected
const view = &mut value;
```

### owned reference operator

Owned references keep `^` tight to the operand.

```ds
const owned = ^value
```

```ds expected
const owned = ^value;
```

### pointer operator

Pointers keep `*` tight to the operand.

```ds
const ptr = *value
```

```ds expected
const ptr = *value;
```

## Increment and Decrement

### prefix increment

Prefix increment stays attached to the identifier.

```ds
++count
```

```ds expected
++count;
```

### prefix decrement

Prefix decrement stays attached to the identifier.

```ds
--count
```

```ds expected
--count;
```

### postfix increment

Postfix increment stays attached to the identifier.

```ds
count++
```

```ds expected
count++;
```

### postfix decrement

Postfix decrement stays attached to the identifier.

```ds
count--
```

```ds expected
count--;
```
