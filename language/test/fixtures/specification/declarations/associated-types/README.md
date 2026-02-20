# Associated Types

Associated types are static type members on class-shaped declarations.

## Coverage

- **Structs**: Associated type aliases, constraints, and projections on structs.
- **Classes**: Associated type aliases, inheritance, and constrained projections on classes.
- **Interfaces**: Associated type requirements, defaults, overrides, and implementor rules.
- **Mixed generics**: Type and static value parameters on generic associated type members.
- **Module boundaries**: Cross module projections and inherited associated defaults.
- **Runtime model**: Associated type members stay type only and are not runtime values.
- **Type index semantics**: Type-space index forms like `T[K]` stay indexed-access types.

## Files

- `structs.md`: Struct associated type declarations and projections.
- `classes.md`: Class associated type declarations and constraints.
- `interfaces.md`: Interface associated types and implementor requirements.
- `modules.md`: Cross module import and export projection coverage.
