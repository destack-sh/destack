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
