# Yield Expressions

Tests for generator `yield` expression formatting.

## Basic Yield

### yield value

Yield expressions keep a space before the value.

```ds
function* gen() { yield 1 }
```

```ds expected
function* gen() {
    yield 1
}
```

### yield star

Yielding a generator uses `yield*` without extra spacing.

```ds
function* gen() { yield* other() }
```

```ds expected
function* gen() {
    yield* other()
}
```
