# Reference Types

> NOTE #Incomplete: implement/mdtest reference types

Explicit reference and value type annotations.

## Coverage

- **Automatic**: `T` - TypeScript behavior
- **Reference**: `&T` - shared access
- **Value**: `^T` - copy semantics

## Example

```ds
function process(data: &Data): void {
    // data is a reference, not copied
}

function modify(data: ^Data): Data {
    // data is a copy, original unchanged
    data
}
```

See also: [ownership/](../../ownership/) for mutability modifiers.
