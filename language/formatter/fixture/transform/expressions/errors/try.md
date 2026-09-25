# Try Expressions

## Try Blocks

### try expression statement

Try expressions used as statements end with semicolons.

```tspp
try operation()
```

```tspp expected
try operation();
```

### try with catch

Catch blocks align with the try block.

```tspp
try { foo() } catch (e) { handle(e) }
```

```tspp expected
try {
    foo()
} catch (e) {
    handle(e)
}
```

### try with typed catch

Catch parameters can have type annotations.

```tspp
try { risky() } catch (error: Error) { handle(error) }
```

```tspp expected
try {
    risky()
} catch (error: Error) {
    handle(error)
}
```

## Try Propagation

### postfix try before binary operators

Postfix `?` binds before binary operators.

```tspp
const size = encode()? + 1
const ready = encode()? && isReady
```

```tspp expected
const size = encode()? + 1;
const ready = encode()? && isReady;
```

### postfix try before type operators

Postfix `?` binds before type assertions.

```tspp
const text = encode()? as string
const valid = encode()? satisfies string
```

```tspp expected
const text = encode()? as string;
const valid = encode()? satisfies string;
```

### postfix try before chain operators

Postfix `?` binds before member and index continuations.

```tspp
const field = encode()?.field
const item = encode()?[0]
```

```tspp expected
const field = encode()?.field;
const item = encode()?[0];
```

### compact ternary

Ternaries require spaces around `?` to disambiguate them from postfix try propagation.

```tspp
const value = a ? b : c
```

```tspp expected
const value = a ? b : c;
```
