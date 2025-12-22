# Types (LanguageFeature::Types)

> NOTE #Incomplete: implement/mdtest Destack type extensions

Type system extensions beyond standard TypeScript.

Destack extends TypeScript's type system with precise primitives, nominal types,
and readable constraints.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `newtypes/` | Nominal (distinct) types |
| `structs/` | Data-oriented object types |
| `primitives/` | Precise numeric types (`int32`, `float64`, etc.) |
| `where/` | Readable generic constraints |
| `refinements/` | Constrained types with validation |
| `references/` | Reference and value type annotations |
| `generics/` | Static parameter type references |

## Example

```ds
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64

struct Point { x: float32, y: float32 }

type User = {
    name: string.minLength(1).maxLength(100),
    age: uint.max(150),
}
```

See [DESIGN.md](../../../../../DESIGN.md#types) for full documentation.
