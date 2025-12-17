# Operators

Tests for operator expression formatting.

## Binary Operators

### arithmetic operators have surrounding spaces

Binary operators without spacing should get spaces added.

```ds
1+2*3-4/5
```

Each operator gets a single space on both sides.

```ds expected
1 + 2 * 3 - 4 / 5;
```

### comparison operators have surrounding spaces

Comparison and logical operators follow the same spacing rules.

```ds
x>1&&y<2||z>=3
```

```ds expected
x > 1 && y < 2 || z >= 3;
```

### assignment operators have surrounding spaces

```ds
x=1
```

```ds expected
x = 1;
```

## Unary Operators

### unary operators have no space

Unary operators should have no space between the operator and operand.

```ds
const x = - 1
const y = ! true
const z = ~ 0xFF
```

The space after the unary operator is removed.

```ds expected
const x = -1;
const y = !true;
const z = ~0xFF;
```

## Ternary Operators

### ternary operators have surrounding spaces

The `?` and `:` in ternary expressions get surrounding spaces.

```ds
x?1:2
```

```ds expected
x ? 1 : 2
```
