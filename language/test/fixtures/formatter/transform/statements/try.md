# Try Statements

Tests for try expressions with catch and finally clauses.

## Basic Try

### try expression statement

Try expressions used as statements end with semicolons.

```ds
try operation()
```

```ds expected
try operation();
```

### try block with catch

Catch blocks align with the try block.

```ds
try { foo() } catch (e) { handle(e) }
```

```ds expected
try {
    foo();
} catch (e) {
    handle(e);
}
```

### try block with catch match

Catch match clauses preserve match formatting.

```ds
try { foo() } catch match (e) { Error(err) => err; _ => null }
```

```ds expected
try {
    foo();
} catch match (e) {
    Error(err) => err
    _ => null
}
```

### catch match pattern matrix

Catch match clauses keep complex patterns and guards readable.

```ds
try { read() } catch match (error) { Network.Timeout { duration } if (duration > 1000) => retry(duration); Validation.Errors([first, ...rest]) => report(first, rest); _ => throw error }
```

```ds expected
try {
    read();
} catch match (error) {
    Network.Timeout { duration } if (duration > 1000) => retry(duration)
    Validation.Errors([first, ...rest]) => report(first, rest)
    _ => throw error
}
```

### catch match block arms

Catch match block arms preserve explicit statements and arm tail values.

```ds
function recover(): Result { try { read() } catch match (error) { Network.Timeout(duration) => { log(duration); retry(duration) }; Validation.Errors(errors) => { report(errors); fallback } } }
```

```ds expected
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

```ds
try {
    read()
} catch match (
    // thrown value
    error
) {
    // timeout branch
    Network.Timeout(/* duration */ duration) => retry(duration);
    // fallback branch
    _ => throw error
}
```

```ds expected
try {
    read();
} catch match (
    // thrown value
    error
) {
    // timeout branch
    Network.Timeout(/* duration */ duration) => retry(duration)
    // fallback branch
    _ => throw error
}
```

### catch tagged tuple pattern

Catch patterns keep tagged tuple destructuring before the handler block.

```ds
try { read() } catch (Result.Err(error, meta = defaultMeta)) { recover(error, meta) }
```

```ds expected
try {
    read();
} catch (Result.Err(error, meta = defaultMeta)) {
    recover(error, meta);
}
```

### catch object pattern with type

Typed catch object patterns keep the annotation inside the catch head.

```ds
try { read() } catch ({ code, message }: Error) { report(code, message) }
```

```ds expected
try {
    read();
} catch ({ code, message }: Error) {
    report(code, message);
}
```

### try block with finally

Finally blocks follow catch blocks.

```ds
try { foo() } catch (e) { handle(e) } finally { cleanup() }
```

```ds expected
try {
    foo();
} catch (e) {
    handle(e);
} finally {
    cleanup();
}
```

### try function tail expression

Try expressions in function tail position preserve try and catch values.

```ds
function read(): number { try { value() } catch (error) { fallback(error) } }
```

```ds expected
function read(): number {
    try {
        value()
    } catch (error) {
        fallback(error)
    }
}
```

### try void function statement tail

Void function bodies keep try and catch terminal expressions statement-valued.

```ds
function read(): void { try { value() } catch (error) { fallback(error) } }
```

```ds expected
function read(): void {
    try {
        value();
    } catch (error) {
        fallback(error);
    }
}
```

### try function tail with finally

Finally blocks stay statement-valued when try and catch branches provide the result.

```ds
function read(): number { try { value() } catch (error) { fallback(error) } finally { cleanup() } }
```

```ds expected
function read(): number {
    try {
        value()
    } catch (error) {
        fallback(error)
    } finally {
        cleanup();
    }
}
```

### try function tail with explicit branch statements

Explicit semicolons inside try and catch branches are preserved.

```ds
function read(): number { try { value(); } catch (error) { fallback(error); } }
```

```ds expected
function read(): number {
    try {
        value();
    } catch (error) {
        fallback(error);
    }
}
```

### try tail comments

Comments before branch tail expressions stay in the branch block.

```ds
function read(): number { try { // cached
value() } catch (error) { // fallback
fallback(error) } }
```

```ds expected
function read(): number {
    try {
        // cached
        value()
    } catch (error) {
        // fallback
        fallback(error)
    }
}
```
