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

## Precedence

These tests document that parentheses are preserved in roundtrip formatting.
The parser wraps these in `Parenthesized` nodes, which the formatter preserves.

### await inside maybe preserves parentheses

```ds
(await foo())?
```

```ds expected
(await foo())?;
```

### unary inside maybe preserves parentheses

```ds
(-x)?
```

```ds expected
(-x)?;
```

### postfix inside maybe needs no extra parentheses

```ds
foo()?.bar?
```

```ds expected
foo()?.bar?;
```

### call inside maybe needs no parentheses

```ds
foo()?
```

```ds expected
foo()?;
```

### binary inside maybe preserves parentheses

```ds
(a + b)?
```

```ds expected
(a + b)?;
```

### ternary inside maybe preserves parentheses

```ds
(cond ? a : b)?
```

```ds expected
(cond ? a : b)?;
```
