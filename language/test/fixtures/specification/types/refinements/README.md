# Refinements

> NOTE #Incomplete: implement/mdtest refinement types

Constrained types with compile-time and runtime validation.

## Coverage

- **Numeric**: `.min(n)`, `.max(n)`, `.positive()`, `.negative()`
- **String**: `.minLength(n)`, `.maxLength(n)`, `.nonEmpty()`
- **Array**: `.minLength(n)`, `.maxLength(n)`, `.nonEmpty()`
- **Composition**: Chain refinements `.min(0).max(100)`
- **Compile-time checks**: When values are provable
- **Runtime validation**: Via `parse()` / `safeParse()`

## Example

```ds
type User = {
    name: string.minLength(1).maxLength(100),
    age: uint.max(150),
}

const percentage: float.min(0).max(100) = 75.5;

// Compile-time check
setAge(200);  // error: 200 > max(150)

// Runtime validation
const validated = parse(uint.max(150), userInput);
```
