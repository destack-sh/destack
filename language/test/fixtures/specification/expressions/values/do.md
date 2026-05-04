# Do

`do` turns a block into an expression.

## expression

### do blocks yield the last expression

> The last expression of a do block is the block value.

```ds
let value: number = do {
    let base = 1;
    base + 2
};
```

### do blocks yield void without a tail expression

> A do block without a trailing expression has type void.

```ds
let value: number = do {
    let base = 1;
};
```

- contains: not assignable

### do blocks use nested conditional tails

> Do blocks still produce a value when the tail comes from nested control flow.

```ds
let value: number = do {
    let base = 1;
    if (base == 1) {
        10
    } else {
        20
    }
};
```

### do block local bindings do not escape block scope

> Bindings declared inside do blocks are not visible outside the block.

```ds
const value = do {
    let scoped = 2;
    scoped
};

scoped satisfies int32;
```

- contains: does not exist

### do blocks can produce tuple values from tail expressions

> Do block tail expressions can yield tuple values directly.

```ds
let value: (int32, int32) = do {
    let left = 1;
    let right = 2;
    (left, right)
};
```
