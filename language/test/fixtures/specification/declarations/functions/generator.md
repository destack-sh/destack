# Generator Functions

Generator functions use `yield` and respect declared `Generator<TYield, TReturn, TNext>` types.

## yield typing

### yield values satisfy the declared yield type

> Yield expressions must satisfy the declared yield type.

```ds libs=es5,es2015.generator,es2015.iterable
function* gen(): Generator<int32, unknown, unknown> {
    yield 1;
}
```

### yield values reject mismatched types

> Yield expressions reject values that do not satisfy the yield type.

```ds libs=es5,es2015.generator,es2015.iterable
function* gen(): Generator<int32, unknown, unknown> {
    yield "no";
}
```

- contains: not assignable

### yield expressions use the next type

> Yield expressions evaluate to the declared next type.

```ds libs=es5,es2015.generator,es2015.iterable
function* gen(): Generator<int32, string, boolean> {
    let next_value: boolean = yield 1;
    return "done";
}
```

### yield expression type mismatch is reported

> Yield expressions must match the declared next type.

```ds libs=es5,es2015.generator,es2015.iterable
function* gen(): Generator<int32, string, boolean> {
    let next_value: number = yield 1;
    return "done";
}
```

- contains: not assignable

## return typing

### generator return values satisfy the return type

> Generator return statements must satisfy the declared return type.

```ds libs=es5,es2015.generator,es2015.iterable
function* gen(): Generator<int32, string, unknown> {
    return "done";
}
```

### generator return values reject mismatched types

> Generator return statements reject values that do not satisfy the return type.

```ds libs=es5,es2015.generator,es2015.iterable
function* gen(): Generator<int32, string, unknown> {
    return 1;
}
```

- contains: not assignable
