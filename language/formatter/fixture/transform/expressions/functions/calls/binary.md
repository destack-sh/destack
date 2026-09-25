# Binary Call Arguments

## Binary Expressions

### short binary expressions stay on one line

Short binary expressions remain on a single line.

```tspp
a + b + c + d
```

```tspp expected
a + b + c + d;
```

### mixed precedence binary expressions

Operator precedence is preserved without added parentheses.

```tspp
a + b * c - d / e
```

```tspp expected
a + b * c - d / e;
```

### long binary expression breaks

Long binary expressions break at operators with all operands at same indentation.

```tspp line-width=30
result = aLongVariableName + anotherLongName + thirdLongName
```

```tspp expected
result = aLongVariableName
    + anotherLongName
    + thirdLongName;
```

### long binary declarator breaks after equals

Long binary declarators also break after `=` when needed.

```tspp line-width=40
const sum = aLongVariableName + anotherLongName + thirdLongName
```

```tspp expected
const sum = aLongVariableName
    + anotherLongName
    + thirdLongName;
```

### binary with logical operators

Logical operators break the same way.

```tspp line-width=40
const isValid = hasPermission && isActive && !isDisabled
```

```tspp expected
const isValid = hasPermission
    && isActive
    && !isDisabled;
```
