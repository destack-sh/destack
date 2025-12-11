# Newtypes

> NOTE #Incomplete: implement/mdtest newtypes

Nominal (distinct) types that prevent mixing semantically different values.

## Coverage

- **Scalar newtypes**: `newtype UserId = int64`
- **Tuple newtypes**: `newtype Point = (float32, float32)`
- **Struct newtypes**: `newtype Config = { debug: boolean }`
- **Construction**: `UserId(42)`, `Point(1.0, 2.0)`, `Config { debug: true }`
- **Pattern matching**: `UserId(n) => ...`

## Example

```ds
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

const id = UserId(42);
const order = OrderId(42);

// Error: UserId is not assignable to OrderId
const bad: OrderId = id;
```
