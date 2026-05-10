# Binary Call Arguments

## Binary Expressions

### short binary expressions stay on one line

Short binary expressions remain on a single line.

```ds
a + b + c + d
```

```ds expected
a + b + c + d;
```

### mixed precedence binary expressions

Operator precedence is preserved without added parentheses.

```ds
a + b * c - d / e
```

```ds expected
a + b * c - d / e;
```

### long binary expression breaks

Long binary expressions break at operators with all operands at same indentation.

```ds line-width=30
result = aLongVariableName + anotherLongName + thirdLongName
```

```ds expected
result =
    aLongVariableName +
    anotherLongName +
    thirdLongName;
```

### long binary declarator breaks after equals

Long binary declarators also break after `=` when needed.

```ds line-width=40
const sum = aLongVariableName + anotherLongName + thirdLongName
```

```ds expected
const sum =
    aLongVariableName +
    anotherLongName +
    thirdLongName;
```

### binary with logical operators

Logical operators break the same way.

```ds line-width=40
const isValid = hasPermission && isActive && !isDisabled
```

```ds expected
const isValid =
    hasPermission &&
    isActive &&
    !isDisabled;
```
