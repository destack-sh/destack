# Parenthesized Expressions

Parentheses fixtures cover removable grouping and semantically required parentheses.

## Spacing

### redundant parentheses are removed

Redundant grouping parentheses are removed rather than preserved.

```tspp
( 1 + 2 )
```

```tspp expected
1 + 2;
```

## Precedence

These tests document the cases where parentheses still matter semantically and must be preserved.

### await inside maybe preserves parentheses

```tspp
(await foo())?
```

```tspp expected
(await foo())?;
```

### unary inside maybe preserves parentheses

```tspp
(-x)?
```

```tspp expected
(-x)?;
```

### postfix inside maybe needs no extra parentheses

```tspp
foo()?.bar?
```

```tspp expected
foo()?.bar?;
```

### call inside maybe needs no parentheses

```tspp
foo()?
```

```tspp expected
foo()?;
```

### binary inside maybe preserves parentheses

```tspp
(a + b)?
```

```tspp expected
(a + b)?;
```

### ternary inside maybe preserves parentheses

```tspp
(cond ? a : b)?
```

```tspp expected
(cond ? a : b)?;
```
