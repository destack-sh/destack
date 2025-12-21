# References

> NOTE #Incomplete: implement/mdtest reference types (&T, &mut T)

Reference types for shared access.

## Coverage

- **Immutable reference**: `&T` or `&const T`
- **Mutable reference**: `&mut T`
- **Automatic**: Plain `T` uses TypeScript behavior

## Example

```ds
function read(data: &Data): void {
    // can read but not modify
}

function modify(data: &mut Data): void {
    // can read and modify
    data.value = 42;
}
```
