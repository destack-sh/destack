# Match Decorators

Tests for decorators on match arms.

## match arms

### match arm decorators are accepted

> Decorators can be applied to match arms.

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

### match arm decorators reject conflicting likely and unlikely

> likely and unlikely decorators cannot be combined on one match arm.

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

- contains: likely and unlikely decorators cannot be combined

### wildcard match arm decorators are accepted

> Decorators can be applied to wildcard match arms.

```ds
declare const value: int | string;

const result = match (value) {
    0 => "zero"
    @cold
    _ => "other"
};

result satisfies string;
```

### duplicate likely decorators on one arm are rejected

> Duplicate branch-prediction decorators are rejected on one match arm.

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

- contains: likely

### match arm decorators reject duplicate cold decorators

> Duplicate decorators on one arm are rejected.

```ds
declare const value: int | string;

const result = match (value) {
    @cold
    @cold
    0 => "zero"
    _ => "other"
};

result satisfies string;
```

- contains: duplicate
