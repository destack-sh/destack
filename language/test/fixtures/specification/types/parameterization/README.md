# Parameterization

Parameterization fixtures cover both TS generic type parameters and TS++ static arguments.

## Generic type parameters

- `infer.md`: conditional infer extraction, distributive behavior, and inference precision.
- `mapped.md`: mapped type construction and key remapping.
- `modifiers.md`: mapped modifier propagation.
- `parameters.md`: type parameter modifiers and const type-parameter behavior.
- `recursion.md`: recursive generic instantiation behavior.
- `utility-types.md`: utility type parity coverage.
- `variance.md`: variance and position rules.

## Static arguments

- `type-aliases.md`: static arguments on type aliases and newtypes.
- `classes.md`: static arguments on classes.
- `interfaces.md`: static arguments on interfaces.
- `structs.md`: static arguments on structs.
- `extensions.md`: static arguments on extensions.
- `value-parameters.md`: comptime value-parameter behavior.

Comptime static arguments are validated as static expressions during Analyze.
