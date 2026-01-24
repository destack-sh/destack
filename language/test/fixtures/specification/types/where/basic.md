# Where Clauses

## assertions

### single constraint is allowed

> Single where assertions are accepted.

```ds
interface Copy {}

function process<T>(value: T): T where T: Copy {
    return value;
}
```

### multiple constraints are allowed

> Multiple where assertions are accepted.

```ds
interface Copy {}
interface Mergeable {}

function merge<T, U>(value: T, other: U): T where (
    T: Copy,
    U: Mergeable
) {
    value;
    other;
    return value;
}
```

### _constraint rejects unsatisfied type arguments

> Type arguments must satisfy where constraints.

```ds
interface Copy {}
interface NotCopy {}

function process<T>(value: T): T where T: Copy {
    return value;
}

const value: NotCopy = {};
process<NotCopy>(value);
```

- contains: constraint

### _multiple constraints reject unsatisfied arguments

> Each where constraint must be satisfied.

```ds
interface Copy {}
interface Mergeable {}

function merge<T, U>(value: T, other: U): T where (
    T: Copy,
    U: Mergeable
) {
    value;
    other;
    return value;
}

const value: Copy = {};
const other: Copy = {};
merge<Copy, Copy>(value, other);
```

- contains: constraint
