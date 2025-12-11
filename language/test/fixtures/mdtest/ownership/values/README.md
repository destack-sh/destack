# Values

> NOTE #Incomplete: implement/mdtest value types (^T, ^var T)

Value types with copy semantics.

## Coverage

- **Immutable value**: `^T` or `^const T`
- **Mutable value**: `^var T`
- **Copy on pass**: Value is copied when passed

## Example

```ds
function process(data: ^Data): Data {
    // data is a copy, original unchanged
    data.value = 42;  // only modifies the copy
    data
}

const original = Data { value: 0 };
const result = process(original);
// original.value is still 0
```
