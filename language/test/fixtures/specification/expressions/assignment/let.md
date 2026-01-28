# Let

Let bindings are mutable.

## Bindings

### let bindings allow assignment

> Let bindings may be reassigned.

```ds
let value: number = 1;
value = 2;
value satisfies number;
```

### let bindings allow compound assignment

> Let bindings support compound assignment operators.

```ds
let value: number = 1;
value += 2;
value satisfies number;
```

### let destructuring allows assignment

> Let bindings created from destructuring are mutable.

```ds
let { count }: { count: number } = { count: 0 };
count = 1;
count satisfies number;
```
