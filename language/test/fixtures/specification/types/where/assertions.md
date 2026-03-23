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

### multiple constraints accept satisfying arguments

> Where constraints allow compatible type arguments.

```ds
interface Copy { copy(): void; }
interface Mergeable { merge(): void; }

function merge<T, U>(value: T, other: U): T where (
    T: Copy,
    U: Mergeable
) {
    value;
    other;
    return value;
}

const value: Copy = { copy() {} };
const other: Mergeable = { merge() {} };
merge<Copy, Mergeable>(value, other);
```

### constraint rejects unsatisfied type arguments

> Type arguments must satisfy where constraints.

```ds
interface Copy { copy(): void; }
interface NotCopy {}

function process<T>(value: T): T where T: Copy {
    return value;
}

const value: NotCopy = {};
process<NotCopy>(value);
```

- contains: not assignable

### multiple constraints reject unsatisfied arguments

> Each where constraint must be satisfied.

```ds
interface Copy { copy(): void; }
interface Mergeable { merge(): void; }

function merge<T, U>(value: T, other: U): T where (
    T: Copy,
    U: Mergeable
) {
    value;
    other;
    return value;
}

const value: Copy = { copy() {} };
const other: Copy = { copy() {} };
merge<Copy, Copy>(value, other);
```

- contains: not assignable

### inferred type arguments must satisfy where constraints

> Constraint checks also apply when type arguments are inferred.

```ds
interface Copy { copy(): void; }

function process<T>(value: T): T where T: Copy {
    return value;
}

process({ copy() {} });
```

### inferred type arguments reject unsatisfied where constraints

> Inferred type arguments are rejected when where constraints are not satisfied.

```ds
interface Copy { copy(): void; }

function process<T>(value: T): T where T: Copy {
    return value;
}

process({ merge() {} });
```

- contains: not assignable
