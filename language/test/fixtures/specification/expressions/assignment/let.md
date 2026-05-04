# Let

Let bindings are mutable.

## bindings

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

### let bindings reject incompatible assignment

> Let bindings still enforce declared type rules.

```ds
let value: number = 1;
value = "no";
```

- contains: not assignable

### let bindings allow update expressions

> Let bindings can be updated through increment and decrement operators.

```ds
let value: number = 1;
value++;
value--;
value satisfies number;
```

### let tuple destructuring bindings remain mutable

> Let tuple destructuring bindings can be reassigned after declaration.

```ds
let (left, right): (number, number) = (1, 2);
left = 3;
right = 4;
left satisfies number;
right satisfies number;
```
