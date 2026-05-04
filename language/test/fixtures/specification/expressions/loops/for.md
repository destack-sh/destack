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

### for allows continue in the loop body

> Continue is accepted in for loop bodies.

```ds
for (let i = 0; i < 3; i++) {
    if (i == 1) {
        continue;
    }
}
```

### for allows break in the loop body

> Break is accepted in for loop bodies.

```ds
for (let i = 0; i < 3; i++) {
    if (i == 1) {
        break;
    }
}
```

### for loop initializer bindings do not escape loop scope

> For-loop initializer bindings are scoped to the loop.

```ds
for (let i = 0; i < 1; i++) {}
i satisfies int32;
```

- contains: does not exist
