# Parenthesized Expressions

Parentheses fixtures cover removable grouping and semantically required parentheses.

## Spacing

### redundant parentheses are removed

Redundant grouping parentheses are removed rather than preserved.

```ds
( 1 + 2 )
```

```ds expected
1 + 2;
```

## Precedence

These tests document the cases where parentheses still matter semantically and must be preserved.

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
