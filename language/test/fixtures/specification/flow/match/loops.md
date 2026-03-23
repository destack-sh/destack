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

### loops inside match allow continue

> Continue inside a nested loop remains valid inside a match arm.

```ds
function match_allows_loop_continue(value: int32): int32 {
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
            result = 2;
        }
    }
    result
}
```

### loop control still rejects direct continue in arms

> Continue still rejects when it is not nested inside a loop.

```ds
function invalid_direct_continue(value: int32): int32 {
    match (value) {
        0 => continue
        _ => 1
    }
}
```

- contains: invalid continue

### loop control still rejects direct break in arms

> Break still rejects when it is not nested inside a loop.

```ds
function invalid_direct_break(value: int32): int32 {
    match (value) {
        0 => break
        _ => 1
    }
}
```

- invalid break to '<none>'