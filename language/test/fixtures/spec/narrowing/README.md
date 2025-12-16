# Narrowing

> NOTE #Incomplete: mdtest type narrowing

Type narrowing and control flow analysis.

## Coverage

- **Type guards**: `typeof`, `instanceof`, custom type guards
- **Truthiness**: Narrowing via boolean coercion
- **Equality**: Narrowing via `===`, `!==`
- **Assignment**: Type refinement through assignment
- **Discriminated unions**: Narrowing via discriminant property

Control flow analysis tracks how types change through conditionals,
assignments, and other control structures.
