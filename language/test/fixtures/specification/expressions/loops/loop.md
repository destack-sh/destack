# Loop Expressions

## loop

### loop yields void

> Loop expressions evaluate to void.

```ds
let value: void = loop {
    break;
};
```

### loop is not assignable to number

> Loop expressions are not assignable to non-void targets.

```ds
let value: number = loop {
    break;
};
```

- contains: not assignable

### labeled loop break is allowed

> Labeled loops can be exited with a matching break.

```ds
let value: void = outer: loop {
    break outer;
};
```
