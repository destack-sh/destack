# Parameter Constraints

Generic parameters can declare constraints directly in the parameter list.

## types

### type parameters accept matching arguments

> A constrained type parameter accepts type arguments that satisfy the constraint.

```ds
interface Copy {
    copy(): void;
}

function clone<T: Copy>(value: T): T {
    return value;
}

const value: Copy = { copy() {} };
clone<Copy>(value);
```

### type parameters reject mismatched arguments

> A constrained type parameter rejects type arguments that do not satisfy the constraint.

```ds
interface Copy {
    copy(): void;
}

interface NotCopy {}

function clone<T: Copy>(value: T): T {
    return value;
}

const value: NotCopy = {};
clone<NotCopy>(value);
```

- contains: not assignable

### inferred arguments must satisfy constraints

> Inferred type arguments are checked against their parameter constraints.

```ds
interface Copy {
    copy(): void;
}

function clone<T: Copy>(value: T): T {
    return value;
}

clone({ copy() {} });
```

### inferred arguments reject unmet constraints

> Inference rejects inferred types that do not satisfy the constraint.

```ds
interface Copy {
    copy(): void;
}

function clone<T: Copy>(value: T): T {
    return value;
}

clone({ merge() {} });
```

- contains: not assignable

## values

### static value parameters accept matching arguments

> Static value parameters can also declare constraints.

```ds
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes = take<4>([1, 2, 3, 4]);
bytes satisfies [uint8; 4];
```

### static value parameters reject mismatched arguments

> Static value arguments must satisfy their declared constraint.

```ds
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

take<"four">([1, 2, 3, 4]);
```

- contains: not assignable
