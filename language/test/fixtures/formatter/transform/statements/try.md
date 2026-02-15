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
    foo()
} catch match (e) {
    Error(err) => err
    _ => null
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
