# Match Expressions

Match expressions participate in control flow typing, narrowing, and result type commitment.
Decorator coverage lives in `decorators.md`.

## Example

```ds
const label = match (state) {
    Ready => "go"
    Loading => "wait"
    Error(e) if e.retryable => "retry"
    Error(e) => `failed: ${e}`
};
```
