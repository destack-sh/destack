# While Expressions

`while` repeats while its boolean condition holds.

## while

### while yields void

While expressions evaluate to void.

```ds
let value: void = while (true) {
    break;
};
```

### while is not assignable to number

While expressions are not assignable to non-void targets.

```ds
let value: number = while (true) {
    break;
};
```

- contains: not assignable

### while allows continue in the loop body

Continue is accepted in while loop bodies.

```ds
let i = 0;
while (i < 3) {
    i = i + 1;
    if (i == 2) {
        continue;
    }
}
```

### while allows break in the loop body

Break is accepted in while loop bodies.

```ds
let i = 0;
while (i < 3) {
    i = i + 1;
    if (i == 2) {
        break;
    }
}
```

### while condition supports narrowing in the body

While conditions contribute control-flow narrowing inside the loop body.

```ds
let value: string | int32 = "ok";
while (value is string) {
    value satisfies string;
    break;
}
```
