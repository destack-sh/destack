# Finally Clauses

## Finally Clauses

### try with finally

Finally can be used without a catch block.

```tspp
try { foo() } finally { cleanup() }
```

```tspp expected
try {
    foo()
} finally {
    cleanup()
}
```

### try with catch and finally

Finally blocks follow catch blocks.

```tspp
try { foo() } catch (e) { handle(e) } finally { cleanup() }
```

```tspp expected
try {
    foo()
} catch (e) {
    handle(e)
} finally {
    cleanup()
}
```

### comments before catch and finally

Keep comments before their clauses.

```tspp
try { foo() }
// recover
catch (error) { handle(error) }
// clean up
finally { cleanup() }
```

```tspp expected
try {
    foo()
}
// recover
catch (error) {
    handle(error)
}
// clean up
finally {
    cleanup()
}
```

### comment after finally

Keep the comment between the keyword and its body.

```tspp
try { foo() } finally
// clean up
{ cleanup() }
```

```tspp expected
try {
    foo()
} finally // clean up
{
    cleanup()
}
```
