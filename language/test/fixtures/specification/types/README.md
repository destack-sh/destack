# Types

Type system extensions beyond standard TypeScript.
Destack extends TypeScript's type system with precise primitives, nominal types, and readable constraints.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `combinators/` | Union and intersection types |
| `enums/` | Enum declarations and backing types |
| `newtypes/` | Nominal (distinct) types |
| `objects/` | Structural object types and interfaces |
| `structs/` | Data-oriented object types |
| `primitives/` | Precise numeric types (`int32`, `float64`, etc.) |
| `where/` | Readable generic constraints |
| `refinements/` | Constrained types with validation |
| `references/` | Reference and value type annotations |
| `generics/` | Static parameter type references |

## Files

- `overview.md`: Nominal types, structs, and refinements

See [DESIGN.md](../../../../../DESIGN.md#types) for full documentation.
