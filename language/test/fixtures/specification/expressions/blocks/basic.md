# Block Basics

## do blocks

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
