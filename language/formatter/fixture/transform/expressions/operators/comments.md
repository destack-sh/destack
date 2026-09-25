# Operator Comments

## Comments in Logical Operators

### comment between logical and operands

Comments between logical operators can cause expansion when they add visual weight.

```tspp line-width=60
const valid = isActive() && /* must have permission */ hasPermission()
```

```tspp expected
const valid = isActive()
    && /* must have permission */ hasPermission();
```

### comments in multiline logical chain

When logical chains break, comments stay with their operands.

```tspp line-width=40
const valid = isActive() && /* perm */ hasPermission() && /* not blocked */ !isBlocked()
```

```tspp expected
const valid = isActive()
    && /* perm */ hasPermission()
    && /* not blocked */ !isBlocked();
```

### comment in nullish coalescing

Comments in nullish coalescing expressions.

```tspp
const value = input ?? /* fallback */ defaultValue
```

```tspp expected
const value = input ?? /* fallback */ defaultValue;
```

## Inline Comments

### comment in binary expression

Comments in binary expressions keep normalized spacing.

```tspp line-width=100
const x = /* pre-A */ A /* A comment */ && B /* B comment */
```

```tspp expected
const x = /* pre-A */ A /* A comment */ && B; /* B comment */
```
