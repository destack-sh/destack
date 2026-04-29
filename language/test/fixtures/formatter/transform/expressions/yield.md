# Yield Expressions

Yield fixtures cover generator yield values, delegation, comments, and member chains.

## Yield Forms

### yield value

Yield expressions keep a space before the value.

```ds
function* gen() { yield 1 }
```

```ds expected
function* gen() {
    yield 1;
}
```

### yield star

Yielding a generator uses `yield*` without extra spacing.

```ds
function* gen() { yield* other() }
```

```ds expected
function* gen() {
    yield* other();
}
```
