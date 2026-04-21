# Overloading

Function and operator overloading with distinct implementations.

## Coverage

- **Function overloading**: Multiple implementations for different parameter types
- **Overload resolution**: Declaration order determines matching
- **Operator overloading**: Via interface implementation (`Add`, `Compare`, etc.)
- **Receiver-based dispatch**: `a + b` becomes `a.add(b)`

## Example

```ds
function parse(input: string): int32 { parseInt(input) }
function parse(input: int32): int32 { input }

parse("42")   // calls first
parse(42)     // calls second
```

Operator overloading requires explicit `implements`:

```ds
extension of Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 { ... }
}

v1 + v2  // becomes v1.add(v2)
```
