# Control Integration

Control integration fixtures cover value-tail behavior across nested branch, match, try, and decorator syntax.

## Branch Tails

### try catch match with if-let tail

Try branches, catch-match arms, and nested if-let branches all preserve value-tail shape.

```ds
function read(): number { try { if (let Some(value) = maybe) { value } else { fallback() } } catch match (error) { Network.Timeout { duration } if (duration > 1000) => retry(duration); Validation.Errors([first, ...rest]) => { report(first, rest); fallback() }; _ => throw error } }
```

```ds expected
function read(): number {
    try {
        if (let Some(value) = maybe) {
            value
        } else {
            fallback()
        }
    } catch match (error) {
        Network.Timeout { duration } if (duration > 1000) => retry(duration)
        Validation.Errors([first, ...rest]) => {
            report(first, rest);
            fallback()
        }
        _ => throw error
    }
}
```

### decorated statements inside nested branches

Statement decorators stay above their statement inside nested control-flow value branches.

```ds
function run(): void { if (ready) { @trace work() } else { try { @fallback recover() } catch (error) { @report handle(error) } } }
```

```ds expected
function run(): void {
    if (ready) {
        @trace
        work()
    } else {
        try {
            @fallback
            recover()
        } catch (error) {
            @report
            handle(error)
        }
    }
}
```

## Comment Boundaries

### comments across nested branch tails

Comments before nested branch tails stay with the expression they describe.

```ds
const value = try {
    // before option
    if (let Some(item) = maybe) {
        // item
        item.value
    } else {
        // fallback
        defaultValue
    }
} catch (error) {
    // recover
    recover(error)
}
```

```ds expected
const value = try {
    // before option
    if (let Some(item) = maybe) {
        // item
        item.value
    } else {
        // fallback
        defaultValue
    }
} catch (error) {
    // recover
    recover(error)
};
```
