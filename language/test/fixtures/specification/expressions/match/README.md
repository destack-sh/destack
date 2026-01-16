# Match Expressions

> NOTE #Incomplete: implement/mdtest match expressions

Pattern matching with exhaustiveness checking.

## Coverage

- **Basic matching**: Match on values, literals
- **Destructuring**: Extract values from structs, tuples, arrays
- **Guards**: `if` conditions on match arms
- **Exhaustiveness**: Compiler ensures all cases are handled
- **Match as expression**: Returns value from matched arm
- **Result typing**: Case bodies produce a union type
- **No breaks**: `break` is not allowed inside match arms
- **No continues**: `continue` is not allowed inside match arms
- **Nested loops**: loops inside match arms can still break
- **Pattern typing**: case patterns must be compatible with the matched value
- **Newtypes**: patterns must use the newtype tag

## Example

```ds
const label = match (state) {
    Ready => "go"
    Loading => "wait"
    Error(e) if e.retryable => "retry"
    Error(e) => `failed: ${e}`
};
```
