# Where

`where` clauses move longer generic constraints out of the parameter list.

## clauses

### where accepts one constraint

> A `where` clause can constrain one type parameter.

```ds
interface Copy {
    copy(): void;
}

function clone<T>(value: T): T where T: Copy {
    return value;
}

const value: Copy = { copy() {} };
clone<Copy>(value);
```

### where accepts multiple constraints

> A `where` clause can constrain several parameters.

```ds
interface Copy {
    copy(): void;
}

interface Mergeable {
    merge(): void;
}

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

### where rejects unmet constraints

> Type arguments must satisfy every `where` constraint.

```ds
interface Copy {
    copy(): void;
}

interface NotCopy {}

function clone<T>(value: T): T where T: Copy {
    return value;
}

const value: NotCopy = {};
clone<NotCopy>(value);
```

- contains: not assignable

### where checks each constraint independently

> Each parameter is checked against its own `where` entry.

```ds
interface Copy {
    copy(): void;
}

interface Mergeable {
    merge(): void;
}

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

## inference

### where checks inferred arguments

> Inferred type arguments must satisfy `where` constraints.

```ds
interface Copy {
    copy(): void;
}

function clone<T>(value: T): T where T: Copy {
    return value;
}

clone({ copy() {} });
```

### where rejects inferred arguments

> Inference fails when an inferred type does not satisfy a `where` constraint.

```ds
interface Copy {
    copy(): void;
}

function clone<T>(value: T): T where T: Copy {
    return value;
}

clone({ merge() {} });
```

- contains: not assignable
