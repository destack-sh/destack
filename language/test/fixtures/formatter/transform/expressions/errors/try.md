# Try Expressions

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
