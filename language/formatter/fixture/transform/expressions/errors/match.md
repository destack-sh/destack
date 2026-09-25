# Catch Match Expressions

## Catch Match Expressions

### try with catch match

Catch match clauses preserve match formatting.

```tspp
try { foo() } catch match (e) { Error(err) => err; _ => null }
```

```tspp expected
try {
    foo()
} catch match (e) {
    Error(err) => err
    _ => null
}
```

### try catch match with if-let tail

Try branches, catch-match arms, and nested if-let branches all preserve value-tail shape.

```tspp
function read(): number { try { if (let Some(value) = maybe) { value } else { fallback() } } catch match (error) { Network.Timeout({ duration }) if (duration > 1000) => retry(duration); Validation.Errors([first, ...rest]) => { report(first, rest); fallback() }; _ => panic("unhandled error") } }
```

```tspp expected
function read(): number {
    try {
        if (let Some(value) = maybe) {
            value
        } else {
            fallback()
        }
    } catch match (error) {
        Network.Timeout({ duration }) if (duration > 1000) => retry(duration)
        Validation.Errors([first, ...rest]) => {
            report(first, rest);
            fallback()
        }
        _ => panic("unhandled error")
    }
}
```

### catch match patterns

Catch match clauses keep patterns and guards structured.

```tspp
try { read() } catch match (error) { Network.Timeout({ duration }) if (duration > 1000) => retry(duration); Validation.Errors([first, ...rest]) => report(first, rest); _ => panic("unhandled error") }
```

```tspp expected
try {
    read()
} catch match (error) {
    Network.Timeout({ duration }) if (duration > 1000) => retry(duration)
    Validation.Errors([first, ...rest]) => report(first, rest)
    _ => panic("unhandled error")
}
```

### catch match block arms

Catch match block arms preserve explicit statements and arm tail values.

```tspp
function recover(): Result { try { read() } catch match (error) { Network.Timeout(duration) => { log(duration); retry(duration) }; Validation.Errors(errors) => { report(errors); fallback } } }
```

```tspp expected
function recover(): Result {
    try {
        read()
    } catch match (error) {
        Network.Timeout(duration) => {
            log(duration);
            retry(duration)
        }
        Validation.Errors(errors) => {
            report(errors);
            fallback
        }
    }
}
```

### catch match comments

Comments around catch match selectors and arms stay attached.

```tspp
try {
    read()
} catch match (
    // failure value
    error
) {
    // timeout branch
    Network.Timeout(/* duration */ duration) => retry(duration);
    // fallback branch
    _ => panic("unhandled error")
}
```

```tspp expected
try {
    read()
} catch match (
    // failure value
    error
) {
    // timeout branch
    Network.Timeout(/* duration */ duration) => retry(duration)
    // fallback branch
    _ => panic("unhandled error")
}
```

### catch tagged tuple pattern

Catch patterns keep tagged tuple destructuring before the handler block.

```tspp
try { read() } catch (Result.Err(error, meta = defaultMeta)) { recover(error, meta) }
```

```tspp expected
try {
    read()
} catch (Result.Err(error, meta = defaultMeta)) {
    recover(error, meta)
}
```

### catch object pattern with type

Typed catch object patterns keep the annotation inside the catch head.

```tspp
try { read() } catch ({ code, message }: Error) { report(code, message) }
```

```tspp expected
try {
    read()
} catch ({ code, message }: Error) {
    report(code, message)
}
```

### catch match with nested patterns and comments

Catch-match patterns keep comments attached while preserving arm value tails.

```tspp
try { read() } catch match (error) { // network
Network.Timeout({ duration }) if (duration > 1000) => retry(duration); // validation
Validation.Errors([first, ...rest]) => { report(first, rest); fallback() }; _ => panic("unhandled error") }
```

```tspp expected
try {
    read()
} catch match (error) {
    // network
    Network.Timeout({ duration }) if (duration > 1000) => retry(duration)
    // validation
    Validation.Errors([first, ...rest]) => {
        report(first, rest);
        fallback()
    }
    _ => panic("unhandled error")
}
```
