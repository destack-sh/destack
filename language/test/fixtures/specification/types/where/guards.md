# Where Guards

## guards

### guard constraints are allowed

> Guard expressions are accepted in where clauses.

```ds
function clamp(value: int32): int32 where value > 0 {
    return value;
}
```
