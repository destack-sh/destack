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

### readonly reference operator

Readonly references keep `&readonly` tight to the operand.

```ds
const view = &readonly value
```

```ds expected
const view = &readonly value;
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

### reference operator comments

Comments after ownership operators group the operand.

```ds
const refs = (& /* borrow */ value, ^ /* own */ value, * /* pointer */ value)
```

```ds expected
const refs = (&(/* borrow */ value), ^(/* own */ value), *(/* pointer */ value));
```

### readonly reference operator comments

Comments after readonly ownership prefixes group the operand.

```ds
const refs = (&readonly /* borrow */ value, *readonly /* pointer */ value)
```

```ds expected
const refs = (&readonly (/* borrow */ value), *readonly (/* pointer */ value));
```

### reference operators in return tail

Ownership operators stay semicolonless when returned as function tail values.

```ds
function borrow(value: Buffer): &readonly Buffer { &readonly value }
```

```ds expected
function borrow(value: Buffer): &readonly Buffer {
    &readonly value
}
```

### reference operators in nested value tail

Ownership operators compose with nested control-flow value tails.

```ds
function borrow(value: Buffer, fallback: Buffer): &readonly Buffer { if (ready) { &readonly value } else { &readonly fallback } }
```

```ds expected
function borrow(value: Buffer, fallback: Buffer): &readonly Buffer {
    if (ready) {
        &readonly value
    } else {
        &readonly fallback
    }
}
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
