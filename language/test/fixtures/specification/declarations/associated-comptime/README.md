# Associated Comptime Constants

Associated comptime constant tests live here.

## Coverage

- **Basics**: Introductory owner, implementor, and projection behavior.
- **Owner kinds**: Classes, structs, interfaces, and extensions.
- **Contracts**: Abstract interface requirements, defaults, and implementor overrides.
- **Projection semantics**: Type level projection and value level projection with compile-time resolvability.
- **Runtime boundary**: Distinguishing `static const` runtime members from `comptime const` associated compile-time members.
- **Static evaluation**: Initializer restrictions and cycle handling.
- **Module boundaries**: Imported associated comptime projections and inherited contracts.
- **Shape modeling**: Vector and tensor style layout composition through associated values and aliases.
- **Type composition**: Conditional and mapped type interactions with associated projections.
- **Type index disambiguation**: Value-space indexes like `T[this.Width]` and `T[Rows]` resolve as fixed-size arrays.

## Files

- `basic.md`: Introductory associated comptime behavior.
- `classes.md`: Class owned associated comptime constants.
- `structs.md`: Struct owned associated comptime constants.
- `interfaces.md`: Interface requirements, defaults, and implementor matching.
- `contracts.md`: Owner and constraint interactions across inheritance and mixed contracts.
- `projections.md`: Projection semantics, resolvability, and runtime usage rules.
- `modules.md`: Cross module resolution and inherited behavior through imports.
- `layouts.md`: Vector and tensor style layout composition through associated members.
- `composition.md`: Conditional and mapped type compositions with associated members.
