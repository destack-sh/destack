# Try Statements

Try expressions preserve branch values while finally clauses remain effect-only.

## Try Blocks

### try expression statement

Try expressions used as statements end with semicolons.

```ds
try operation()
```

```ds expected
try operation();
```

### try with catch

Catch blocks align with the try block.

```ds
try { foo() } catch (e) { handle(e) }
```

```ds expected
try {
    foo()
} catch (e) {
    handle(e)
}
```

### try with typed catch

Catch parameters can have type annotations.

```ds
try { risky() } catch (error: Error) { handle(error) }
```

```ds expected
try {
    risky()
} catch (error: Error) {
    handle(error)
}
```

### try with catch match

Catch match clauses preserve match formatting.

```ds
try { foo() } catch match (e) { Error(err) => err; _ => null }
```

```ds expected
try {
    foo()
} catch match (e) {
    Error(err) => err
    _ => null
}
```

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

### catch match patterns

Catch match clauses keep patterns and guards structured.

```ds
try { read() } catch match (error) { Network.Timeout { duration } if (duration > 1000) => retry(duration); Validation.Errors([first, ...rest]) => report(first, rest); _ => throw error }
```

```ds expected
try {
    read()
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
    read()
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
    read()
} catch (Result.Err(error, meta = defaultMeta)) {
    recover(error, meta)
}
```

### catch object pattern with type

Typed catch object patterns keep the annotation inside the catch head.

```ds
try { read() } catch ({ code, message }: Error) { report(code, message) }
```

```ds expected
try {
    read()
} catch ({ code, message }: Error) {
    report(code, message)
}
```

### try with finally

Finally can be used without a catch block.

```ds
try { foo() } finally { cleanup() }
```

```ds expected
try {
    foo()
} finally {
    cleanup();
}
```

### try with catch and finally

Finally blocks follow catch blocks.

```ds
try { foo() } catch (e) { handle(e) } finally { cleanup() }
```

```ds expected
try {
    foo()
} catch (e) {
    handle(e)
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

### try in void function tail

Try and catch branches keep expression tails in void functions.

```ds
function read(): void { try { value() } catch (error) { fallback(error) } }
```

```ds expected
function read(): void {
    try {
        value()
    } catch (error) {
        fallback(error)
    }
}
```

### try function tail with finally

Finally blocks stay effect-only when try and catch branches provide the result.

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

### try initializer value

Try initializer values with catch clauses expand branch blocks.

```ds
const payload = try { readPayload(source) } catch (error) { recoverPayload(error) }
```

```ds expected
const payload = try {
    readPayload(source)
} catch (error) {
    recoverPayload(error)
};
```

### try object property value

Try object property values with catch clauses expand branch blocks.

```ds
const envelope = { payload: try { readPayload(source) } catch (error) { recoverPayload(error) } }
```

```ds expected
const envelope = {
    payload: try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
};
```

### try argument value

Try argument values with catch clauses expand branch blocks.

```ds
render(try { readPayload(source) } catch (error) { recoverPayload(error) })
```

```ds expected
render(
    try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
);
```

### try await operand value

Try await operands with catch clauses expand branch blocks.

```ds
const awaited = await (try { load(source) } catch (error) { recover(error) })
```

```ds expected
const awaited = await (try {
    load(source)
} catch (error) {
    recover(error)
});
```

### long try initializer value

Long try initializer values expand branch bodies.

```ds line-width=80
const payload = try { const raw = readCachedPayload(cacheKey, options); parsePayload(raw, schema, options) } catch (error) { const diagnostic = diagnostics.describe(error, context.locale); recoverPayload(diagnostic, fallbackPayload, options) }
```

```ds expected
const payload = try {
    const raw = readCachedPayload(cacheKey, options);
    parsePayload(raw, schema, options)
} catch (error) {
    const diagnostic = diagnostics.describe(error, context.locale);
    recoverPayload(diagnostic, fallbackPayload, options)
};
```

### long try await operand value

Long try await operands expand branch bodies.

```ds line-width=80
const awaited = await (try { loadAsync(source) } catch (error) { recoverAsync(error) })
```

```ds expected
const awaited = await (try {
    loadAsync(source)
} catch (error) {
    recoverAsync(error)
});
```

### multiline try initializer value

Manually broken try initializer values keep the expanded branch shape.

```ds
const payload = try {
    readPayload(source)
} catch (error) {
    recoverPayload(error)
}
```

```ds expected
const payload = try {
    readPayload(source)
} catch (error) {
    recoverPayload(error)
};
```

### multiline try argument value

Manually broken try argument values keep the expanded branch shape.

```ds
render(try {
    readPayload(source)
} catch (error) { recoverPayload(error) })
```

```ds expected
render(
    try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
);
```
