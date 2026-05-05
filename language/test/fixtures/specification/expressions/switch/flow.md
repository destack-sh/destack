# Switch Flow

## breaks

### switch breaks target the innermost switch

> Switch breaks cannot carry values, even inside loops.

```ds
function invalid_switch_break_value_in_loop(value: int32): int32 {
    loop {
        switch (value) {
            case 0:
                break 1;
            default:
                break;
        }
        break;
    }
    0
}
```

- contains: switch break cannot have a value

### labeled break can target a switch

> Labeled breaks can exit the labeled switch.

```ds
function labeled_break_switch(value: int32): int32 {
    let count: int32 = 0;
    outer: switch (value) {
        case 0:
            count = 1;
            break outer;
        default:
            count = 2;
            break;
    }
    count
}
```

### labeled break can target a loop

> Labeled breaks can exit a loop from inside a switch.

```ds
function labeled_break_loop_from_switch(value: int32): int32 {
    let count: int32 = 0;
    outer: loop {
        switch (value) {
            case 0:
                count = 1;
                break outer;
            default:
                count = 2;
                break;
        }
        count = count + 1;
        break;
    }
    count
}
```

## continues

### switch does not allow continue

> Continue is only accepted in loops.

```ds
function invalid_switch_continue(value: int32): int32 {
    switch (value) {
        case 0:
            continue;
        default:
            break;
    }
    0
}
```

- contains: invalid continue

### continue inside switch targets the loop

> Continue inside a switch within a loop continues the loop.

```ds
function continue_in_switch(value: int32): int32 {
    let count: int32 = 0;
    loop {
        switch (value) {
            case 0:
                count = count + 1;
                continue;
            default:
                break;
        }
        break;
    }
    count
}
```

### labeled continue can target a loop

> Labeled continues can target loops from inside a switch.

```ds
function labeled_continue_loop_from_switch(value: int32): int32 {
    let count: int32 = 0;
    outer: loop {
        switch (value) {
            case 0:
                count = count + 1;
                continue outer;
            default:
                break;
        }
        break;
    }
    count
}
```

### labeled continue cannot target a switch

> Continue requires a loop label, even when a switch is labeled.

```ds
function invalid_labeled_continue_switch(value: int32): int32 {
    outer: switch (value) {
        case 0:
            continue outer;
        default:
            break;
    }
    0
}
```

- contains: invalid continue

## guards

### switch rejects guards

> Switch cases reject guards.

```ds
function invalid_switch_guard(value: int32): int32 {
    switch (value) {
        case 0 if value > 0:
            break;
        default:
            break;
    }
    0
}
```

- contains: switch cases do not support guards

## value

### switch does not yield a value

> Switch is a statement.

```ds
function invalid_switch_expression(value: int32): int32 {
    const result: int32 = switch (value) {
        case 0:
            break;
        default:
            break;
    };
    result
}
```

- contains: not assignable
