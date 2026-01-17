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
