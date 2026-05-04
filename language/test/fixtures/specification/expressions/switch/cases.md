# Switch Statements

## syntax

### switch selects a matching case

> Switch statements run the first matching case.

```ds
function describe(day: int32): string {
    let label: string = "unknown";
    switch (day) {
        case 0:
            label = "sun";
            break;
        default:
            label = "other";
    }
    label
}
```

### switch falls through without break

> Switch cases fall through when no break is present.

```ds
function classify(day: int32): int32 {
    let result: int32 = 0;
    switch (day) {
        case 0:
        case 6:
            result = 1;
            break;
        case 1:
        case 2:
            result = 2;
            break;
        default:
            result = 3;
    }
    result
}
```

## errors

### switch rejects break values

> Switch breaks cannot carry a value.

```ds
function invalidBreak(day: int32): int32 {
    switch (day) {
        case 0:
            break 1;
        default:
            break;
    }
    0
}
```

- contains: switch break cannot have a value

### switch cases require expression patterns

> Switch cases do not accept structural patterns.

```ds
function invalidCase(value: int32): int32 {
    switch (value) {
        case (1, 2):
            break;
        default:
            break;
    }
    0
}
```

- contains: switch cases require expression patterns

### switch cases do not accept wildcards

> Use `default` instead of a wildcard case.

```ds
function invalidWildcard(value: int32): int32 {
    switch (value) {
        case _:
            break;
        default:
            break;
    }
    0
}
```

- contains: switch cases require expression patterns

### switch cases must match the switch type

> Case expressions must be assignable to the switch value.

```ds
function invalidCaseType(value: int32): int32 {
    switch (value) {
        case "hi":
            break;
        default:
            break;
    }
    0
}
```

- contains: not assignable
