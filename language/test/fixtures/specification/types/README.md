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
| `references/` | Reference and value type annotations |
| `operators/` | Type-level operators like `keyof` and `typeof` |
| `generics/` | Generic inference and mapped/utility types |
| `static-arguments/` | Static arguments for type and value parameters |
| `template-literals/` | Template literal types and inference |

## Files

- `overview.md`: Nominal types and structs

See [DESIGN.md](../../../../../DESIGN.md#types) for full documentation.
