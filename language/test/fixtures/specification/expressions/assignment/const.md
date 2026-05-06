# Const

Const bindings are immutable, but member assignment is allowed.

## bindings

### const bindings reject assignment

Const bindings cannot be reassigned.

```ds
const value: number = 1;
value = 2;
```

- contains: immutable binding

### const bindings reject compound assignment

Const bindings cannot use compound assignment operators.

```ds
const value: number = 1;
value += 1;
```

- contains: immutable binding

### const destructuring rejects assignment

Const bindings created from destructuring are immutable.

```ds
const { count }: { count: number } = { count: 0 };
count = 1;
```

- contains: immutable binding

## members

### const bindings allow member assignment

Const bindings do not freeze object members.

```ds
const state: { count: number } = { count: 0 };
state.count = 1;
state.count satisfies number;
```

### const bindings respect readonly properties

Readonly properties cannot be assigned.

```ds
const state: { readonly count: number } = { count: 0 };
state.count = 1;
```

- contains: cannot assign to readonly property 'count'
