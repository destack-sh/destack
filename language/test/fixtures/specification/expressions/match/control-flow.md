# Match Control Flow

## Control Flow

### match does not allow continue

> Continue is invalid inside match arms.

```ds
function invalid_match_continue(value: int32): int32 {
    const result = match (value) {
        0 => continue
        _ => 1
    };
    result
}
```

- contains: invalid continue

### loops inside match allow break

> Break inside a nested loop remains valid inside a match arm.

```ds
function match_allows_loop_break(value: int32): int32 {
    let result: int32 = 0;
    match (value) {
        0 => {
            loop {
                break;
            }
            result = 1;
        }
        _ => {
            result = 2;
        }
    }
    result
}
```
