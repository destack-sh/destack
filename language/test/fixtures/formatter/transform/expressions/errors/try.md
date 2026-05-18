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

## Try Propagation

### postfix try before binary operators

Postfix `?` binds before binary operators.

```ds
const size = encode()? + 1
const ready = encode()? && isReady
```

```ds expected
const size = encode()? + 1;
const ready = encode()? && isReady;
```

### postfix try before type operators

Postfix `?` binds before type assertions.

```ds
const text = encode()? as string
const valid = encode()? satisfies string
```

```ds expected
const text = encode()? as string;
const valid = encode()? satisfies string;
```

### postfix try before chain operators

Postfix `?` binds before member and index continuations.

```ds
const field = encode()?.field
const item = encode()?[0]
```

```ds expected
const field = encode()?.field;
const item = encode()?[0];
```

### compact ternary

Expression operands after `?` keep ternary shape.

```ds
const value = a?b:c
```

```ds expected
const value = a ? b : c;
```
