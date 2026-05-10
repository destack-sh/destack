# Operator Comments

## Comments in Logical Operators

### comment between logical and operands

Comments between logical operators can cause expansion when they add visual weight.

```ds line-width=60
const valid = isActive() && /* must have permission */ hasPermission()
```

```ds expected
const valid =
    isActive() &&
    /* must have permission */ hasPermission();
```

### comments in multiline logical chain

When logical chains break, comments stay with their operands.

```ds line-width=40
const valid = isActive() && /* perm */ hasPermission() && /* not blocked */ !isBlocked()
```

```ds expected
const valid =
    isActive() &&
    /* perm */ hasPermission() &&
    /* not blocked */ !isBlocked();
```

### comment in nullish coalescing

Comments in nullish coalescing expressions.

```ds
const value = input ?? /* fallback */ defaultValue
```

```ds expected
const value = input ?? /* fallback */ defaultValue;
```

## Inline Comments

### comment in binary expression

Comments in binary expressions keep normalized spacing.

```ds line-width=100
const x = /* pre-A */ A /* A comment */ && B /* B comment */
```

```ds expected
const x = /* pre-A */ A /* A comment */ && B; /* B comment */
```
