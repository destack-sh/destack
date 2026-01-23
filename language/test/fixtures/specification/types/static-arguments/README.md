# Static Arguments

Static arguments supply type and comptime value parameters for type references.
Comptime value parameters must be marked with `comptime` in the static parameter list.
Static arguments are resolved during Analyze and must be static expressions.

## Coverage

Coverage areas include:

- Type parameters on aliases, classes, structs, and interfaces.
- Comptime value parameters on aliases, classes, structs, and interfaces.
- Defaults for static type and value parameters.
- Extension parameter mapping and defaults.
- Static argument reuse through type aliases.
