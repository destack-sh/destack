# Composite Types

> NOTE #Incomplete: implement union/tuple type evaluation (tests in staging/)

Union types, intersection types, and tuple types.

## Coverage

- **Union types**: `A | B`, discriminated unions
- **Intersection types**: `A & B`
- **Tuple types**: Fixed-length typed arrays `[A, B, C]`

Union and intersection types are TypeScript's primary way of combining types.
Tuples provide fixed-length, position-typed arrays.
