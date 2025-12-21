# Mutability

> NOTE #Incomplete: implement/mdtest mutability modifiers

Explicit mutability control.

## Coverage

- **const**: Immutable binding/reference
- **var**: Mutable binding/reference
- **Combinations**: `&const T`, `&mut T`, `^const T`, `^var T`

## Example

```ds
&const T     // immutable reference (default for &T)
&mut T       // mutable reference
^const T     // immutable value (default for ^T)
^var T       // mutable value
```
