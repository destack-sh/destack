# Switch Statements

Switch statements follow TypeScript semantics with fallthrough and explicit breaks.

## Coverage

- **Basic switch**: case selection and default handling
- **Fallthrough**: execution continues into the next case without `break`
- **Break**: `break` is required to stop and cannot carry a value
- **Continue**: `continue` is only valid in loops
- **Labeled breaks**: labeled breaks can exit a switch
- **Statement typing**: switch does not yield a value
- **Case selectors**: switch cases require expression patterns
- **Case typing**: case expressions must be compatible with the switch value
- **Guards**: switch cases do not support guards

## Example

```ds
switch (value) {
    case 1:
        print("one");
        break;
    default:
        print("other");
}
```
