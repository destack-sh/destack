# Match Expressions

## Control Flow

### match does not allow break

> Break is invalid inside match arms.

```ds
function invalid_match_break(value: int32): int32 {
    const result = match (value) {
        0 => break
        _ => 1
    };
    result
}
```

- contains: invalid break
