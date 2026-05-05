# Match Flow

## control flow

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

### nested loops allow break

> Break can target a loop nested inside a match arm.

```ds
function nested_loop_break(value: int32): int32 {
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

### nested loops allow continue

> Continue can target a loop nested inside a match arm.

```ds
function nested_loop_continue(value: int32): int32 {
    let result: int32 = 0;
    match (value) {
        0 => {
            let current: int32 = 0;
            while (current < 2) {
                current = current + 1;
                continue;
            }
            result = current;
        }
        _ => {
            result = 3;
        }
    }
    result
}
```
