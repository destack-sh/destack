# While Expressions

## while

### while yields void

> While expressions evaluate to void.

```ds
let value: void = while (true) {
    break;
};
```

### while is not assignable to number

> While expressions are not assignable to non-void targets.

```ds
let value: number = while (true) {
    break;
};
```

- contains: not assignable
