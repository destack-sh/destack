# Yield Expressions

Yield fixtures cover generator yield values, delegation, comments, and member chains.

## Yield Forms

### yield value

Yield expressions keep a space before the value.

```tspp
function* gen() { yield 1 }
```

```tspp expected
function* gen() {
    yield 1;
}
```

### yield star

Yielding a generator uses `yield*` without extra spacing.

```tspp
function* gen() { yield* other() }
```

```tspp expected
function* gen() {
    yield* other();
}
```
