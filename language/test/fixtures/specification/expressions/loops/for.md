# For Expressions

## for

### for yields void

> For expressions evaluate to void.

```ds
let value: void = for (let i = 0; i < 1; i++) {
    i;
};
```

### for is not assignable to number

> For expressions are not assignable to non-void targets.

```ds
let value: number = for (let i = 0; i < 1; i++) {
    i;
};
```

- contains: not assignable
