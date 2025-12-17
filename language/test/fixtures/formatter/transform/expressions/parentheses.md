# Parenthesized Expressions

Tests for parenthesized expression formatting.

## Spacing

### parentheses have no internal spacing

Spaces after `(` and before `)` should be removed.

```ds
( 1 + 2 )
```

```ds expected
(1 + 2);
```
