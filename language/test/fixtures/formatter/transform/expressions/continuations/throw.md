# Throw Statements

## Throw

### throw expression

Throw statements raise an exception with the given value.

```ds
throw new Error("message")
```

```ds expected
throw new Error("message");
```

### throw string

Strings can be thrown directly.

```ds
throw "error"
```

```ds expected
throw "error";
```
