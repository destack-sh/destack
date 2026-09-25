# Unary Expressions

Unary fixtures cover prefix operators, ownership operators, comments, and precedence grouping.

## Prefix Operators

### logical not

Logical not attaches directly to the operand.

```tspp
!isReady
```

```tspp expected
!isReady;
```

### numeric negation

Unary minus attaches directly to the operand.

```tspp
-total
```

```tspp expected
-total;
```

### bitwise not

Bitwise not attaches directly to the operand.

```tspp
~mask
```

```tspp expected
~mask;
```

### await operator

Await keeps a space before the operand.

```tspp
await fetchData()
```

```tspp expected
await fetchData();
```

## TS++ Reference Operators

### reference operator

References keep the `&` tight to the operand.

```tspp
const view = &value
```

```tspp expected
const view = &value;
```

### readonly reference operator

Readonly references keep `&readonly` tight to the operand.

```tspp
const view = &readonly value
```

```tspp expected
const view = &readonly value;
```

### pointer operator

Pointers keep `*` tight to the operand.

```tspp
const ptr = *value
```

```tspp expected
const ptr = *value;
```

### reference operators in return tail

Ownership operators stay semicolonless when returned as function tail values.

```tspp
function borrow(value: Buffer): &readonly Buffer { &readonly value }
```

```tspp expected
function borrow(value: Buffer): &readonly Buffer {
    &readonly value
}
```

### reference operators in nested value tail

Ownership operators compose with nested control-flow value tails.

```tspp
function borrow(value: Buffer, fallback: Buffer): &readonly Buffer { if (ready) { &readonly value } else { &readonly fallback } }
```

```tspp expected
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

```tspp
++count
```

```tspp expected
++count;
```

### prefix decrement

Prefix decrement stays attached to the identifier.

```tspp
--count
```

```tspp expected
--count;
```

### postfix increment

Postfix increment stays attached to the identifier.

```tspp
count++
```

```tspp expected
count++;
```

### postfix decrement

Postfix decrement stays attached to the identifier.

```tspp
count--
```

```tspp expected
count--;
```
