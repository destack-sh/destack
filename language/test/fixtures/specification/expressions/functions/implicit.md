# Implicit Types

Strict functions reject implicit `any`.

## parameters

### parameters require types

> Parameters without annotations or defaults are implicit any and are rejected.

```ds
function handle(value) {
}
```

- contains: implicit any type

### parameter defaults infer parameter types

> Defaults provide an inferred parameter type.

```ds
function handle(value = 1) {
    value satisfies number;
}
```

## bindings

### uninitialized bindings require annotations

> Bindings without annotations or initializers are implicit any.

```ds
let pending;
```

- contains: implicit any type
