# Finally Clauses

## Finally Clauses

### try with finally

Finally can be used without a catch block.

```ds
try { foo() } finally { cleanup() }
```

```ds expected
try {
    foo()
} finally {
    cleanup()
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
    cleanup()
}
```
