# Generator Functions

Generator functions use `yield` and respect declared `Generator<Y, R, N>` types.

## yield typing

### yield values satisfy the declared yield type

Yield expressions must satisfy the declared yield type.

```ds
function* gen(): Generator<int32> {
    yield 1;
}
```

### yield values reject mismatched types

Yield expressions reject values that do not satisfy the yield type.

```ds
function* gen(): Generator<int32> {
    yield "no";
}
```

- contains: not assignable

### yield expressions use the next type

Yield expressions evaluate to the declared next type.

```ds
function* gen(): Generator<int32, string, boolean> {
    let nextValue: boolean = yield 1;
    return "done";
}
```

### yield expression type mismatch is reported

Yield expressions must match the declared next type.

```ds
function* gen(): Generator<int32, string, boolean> {
    let nextValue: number = yield 1;
    return "done";
}
```

- contains: not assignable

## return typing

### generator return values satisfy the return type

Generator return statements must satisfy the declared return type.

```ds
function* gen(): Generator<int32, string> {
    return "done";
}
```

### generator return values reject mismatched types

Generator return statements reject values that do not satisfy the return type.

```ds
function* gen(): Generator<int32, string> {
    return 1;
}
```

- contains: not assignable
