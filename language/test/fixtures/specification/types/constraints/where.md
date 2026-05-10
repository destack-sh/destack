# Where

`where` clauses move longer generic constraints out of the parameter list.

## clauses

### where accepts one constraint

A `where` clause can constrain one type parameter.

```ds
interface Readable {
    read(): void;
}

function clone<T>(value: T): T where T: Readable {
    return value;
}

const value: Readable = {
    read() {},
};
clone<Readable>(value);
```

### where accepts multiple constraints

A `where` clause can constrain several parameters.

```ds
interface Readable {
    read(): void;
}

interface Mergeable {
    merge(): void;
}

function merge<T, U>(value: T, other: U): T where (T: Readable, U: Mergeable) {
    value;
    other;
    return value;
}

const value: Readable = {
    read() {},
};
const other: Mergeable = {
    merge() {},
};

merge<Readable, Mergeable>(value, other);
```

### where rejects unmet constraints

Type arguments must satisfy every `where` constraint.

```ds
interface Readable {
    read(): void;
}

interface NotReadable {}

function clone<T>(value: T): T where T: Readable {
    return value;
}

const value: NotReadable = {};
clone<NotReadable>(value);
```

- contains: not assignable

### where checks each constraint independently

Each parameter is checked against its own `where` entry.

```ds
interface Readable {
    read(): void;
}

interface Mergeable {
    merge(): void;
}

function merge<T, U>(value: T, other: U): T where (T: Readable, U: Mergeable) {
    value;
    other;
    return value;
}

const value: Readable = {
    read() {},
};
const other: Readable = {
    read() {},
};

merge<Readable, Readable>(value, other);
```

- contains: not assignable

## inference

### where checks inferred arguments

Inferred type arguments must satisfy `where` constraints.

```ds
interface Readable {
    read(): void;
}

function clone<T>(value: T): T where T: Readable {
    return value;
}

clone({
    read() {},
});
```

### where rejects inferred arguments

Inference rejects inferred types that do not satisfy a `where` constraint.

```ds
interface Readable {
    read(): void;
}

function clone<T>(value: T): T where T: Readable {
    return value;
}

clone({
    merge() {},
});
```

- contains: not assignable
