# Parameter Constraints

Generic parameters can declare constraints directly in the parameter list.

## types

### type parameters accept matching arguments

A constrained type parameter accepts type arguments that satisfy the constraint.

```ds
interface Readable {
    read(): void;
}

function clone<T: Readable>(value: T): T {
    return value;
}

const value: Readable = {
    read() {},
};
clone<Readable>(value);
```

### type parameters reject mismatched arguments

A constrained type parameter rejects type arguments that do not satisfy the constraint.

```ds
interface Readable {
    read(): void;
}

interface NotReadable {}

function clone<T: Readable>(value: T): T {
    return value;
}

const value: NotReadable = {};
clone<NotReadable>(value);
```

- contains: not assignable

### inferred arguments must satisfy constraints

Inferred type arguments are checked against their parameter constraints.

```ds
interface Readable {
    read(): void;
}

function clone<T: Readable>(value: T): T {
    return value;
}

clone({
    read() {},
});
```

### inferred arguments reject unmet constraints

Inference rejects inferred types that do not satisfy the constraint.

```ds
interface Readable {
    read(): void;
}

function clone<T: Readable>(value: T): T {
    return value;
}

clone({
    merge() {},
});
```

- contains: not assignable

## values

### static value parameters accept matching arguments

Static value parameters can also declare constraints.

```ds
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes = take<4>([1, 2, 3, 4]);
bytes satisfies [uint8; 4];
```

### static value parameters reject mismatched arguments

Static value arguments must satisfy their declared constraint.

```ds
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

take<"four">([1, 2, 3, 4]);
```

- contains: not assignable
