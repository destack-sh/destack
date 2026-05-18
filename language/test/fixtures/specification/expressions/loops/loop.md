# Loop Expressions

## loop

### loop yields void

Loop expressions evaluate to void.

```ds
let value: void = loop {
    break;
};
```

### loop is not assignable to number

Loop expressions are not assignable to non-void targets.

```ds
let value: number = loop {
    break;
};
```

- contains: not assignable

### labeled loop break is allowed

Labeled loops can be exited with a matching break.

```ds
let value: void = outer: loop {
    break outer;
};
```

### loop break values determine the result type

Loop expressions use break values to determine their result type.

```ds
let value: number = loop {
    break (1);
};
```

### loop break values are required to satisfy the target type

Loop break values must satisfy the expected type.

```ds
let value: string = loop {
    break (1);
};
```

- contains: not assignable

### loop breaks without values contribute void

Breaks without values contribute void to the loop result.

```ds
let value: number | void = loop {
    if (true) {
        break (1);
    }
    break;
};
```
