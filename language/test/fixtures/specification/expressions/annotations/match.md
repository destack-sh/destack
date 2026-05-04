# Match Annotations

Match annotations attach to individual arms.

## arms

### arm hints preserve match result typing

> Arm annotations do not change the arm expression type.

```ds
declare const value: int | string;

const result = match (value) {
    @cold
    0 => "zero"
    @likely
    _ => "other"
};

result satisfies string;
```

### wildcard arms accept annotations

> Wildcard arms can carry annotations.

```ds
declare const value: int | string;

const result = match (value) {
    0 => "zero"
    @cold
    _ => "other"
};

result satisfies string;
```

## validation

### conflicting arm hints are rejected

> One match arm cannot carry contradictory branch hints.

```ds
declare const value: int | string;

const result = match (value) {
    @likely
    @unlikely
    0 => "zero"
    _ => "other"
};

result satisfies string;
```

- contains: likely and unlikely

### duplicate arm hints are rejected

> One match arm cannot carry the same branch hint twice.

```ds
declare const value: int | string;

const result = match (value) {
    @likely
    @likely
    0 => "zero"
    _ => "other"
};

result satisfies string;
```

- duplicate
