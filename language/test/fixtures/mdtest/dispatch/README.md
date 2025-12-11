# Dispatch (LanguageFeature::Dispatch)

Type-based dispatch: extensions and overloading.

TypeScript has parametric polymorphism (generics) but no type-based dispatch.
Destack adds extensions and real overloading.

## Coverage

### Extensions

Add methods to existing types without modifying them:

```ds
extension Vector2 {
    magnitude(): float32 { (this.x * this.x + this.y * this.y).sqrt() }
}
```

Extension visibility:
- **Same file as type**: Automatically visible wherever the type is used
- **Anonymous on foreign type**: Only visible in the defining file
- **Named on foreign type**: Must be explicitly imported

### Overloading

Real function and operator overloading with distinct implementations:

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

extension Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}
```

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `extensions/` | Extension declarations and visibility |
| `overloading/` | Function and operator overloading |

See [DESIGN.md](../../../../../DESIGN.md#dispatch) for full documentation.
