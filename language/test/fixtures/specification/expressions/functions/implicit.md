# Implicit Types

Strict functions reject missing parameter and local types.

## parameters

### parameters require types

Parameters without annotations or defaults are rejected.

```ds
function handle(value) {}
```

- contains: implicit any type

### parameter defaults infer parameter types

Defaults provide an inferred parameter type.

```ds
function handle(value = 1) {
    value satisfies number;
}
```

## bindings

### uninitialized bindings require annotations

Bindings without annotations or initializers are rejected.

```ds
let pending;
```

- contains: implicit any type
