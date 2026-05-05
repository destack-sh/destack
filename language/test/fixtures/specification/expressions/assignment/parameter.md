# Parameter Bindings

## reassignment

### parameter bindings are mutable

> Parameters can be reassigned.

```ds
function bump(x: number): void {
    x = 2;
    x satisfies number;
}
```

### parameter assignments enforce declared types

> Reassignment checks the parameter type.

```ds
function bump(x: number): void {
    x = "no";
}
```

- contains: not assignable

### parameter bindings allow compound assignment

> Parameter bindings support compound assignment operators.

```ds
function bump(x: number): number {
    x += 2;
    x
}
```
