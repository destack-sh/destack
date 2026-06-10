# types/

Destack keeps TypeScript's type forms - `type`, `interface`, and `class` work as you expect - and adds precise primitives, value types, and nominality.
Where TypeScript projects encode an invariant in conventions and validation code, Destack usually has a direct spelling.

For example, the TS ecosystem fakes nominal ids with branding hacks:

```ts
type UserId = number & { readonly __brand: unique symbol };
```

In Destack the same intent is one declaration, constructed explicitly and erased at runtime:

```ds
newtype UserId = uint64;
const id = UserId(7);
```

The same idea repeats across the chapter: numbers carry exact widths (`uint8` is one byte, and the compiler checks the bounds), ranges work in type position (`1..=65535`), and structs are value types with known layout.
The files below cover the type system roughly in DESIGN order; `variance.ds` and `readonly.ds` genuinely fail to compile, and the diagnostics view shows what the compiler reports.

| file | shows |
| --- | --- |
| [`primitives.ds`](primitives.ds) | exact-width integers and floats, `char`, statically checked literal bounds |
| [`intervals.ds`](intervals.ds) | ranges in type position, narrowing runtime values through patterns |
| [`newtypes.ds`](newtypes.ds) | nominal identity over a backing representation, erased in the `.js` view |
| [`structs.ds`](structs.ds) | nominal value types, struct construction, spreads |
| [`enums.ds`](enums.ds) | nominal constants with members and exhaustive matching |
| [`unions.ds`](unions.ds) | discriminated unions with `newtype` and the `Tagged` derive |
| [`collections.ds`](collections.ds) | slices, fixed arrays, tuples, and their patterns |
| [`extensions.ds`](extensions.ds) | adding members to any nominal type, statically resolved |
| [`generics.ds`](generics.ds) | type and `comptime` value parameters, visible monomorphization |
| [`constraints.ds`](constraints.ds) | inline bounds and `where` clauses over associated members |
| [`variance.ds`](variance.ds) | derived variance, the mutable covariance hole as a compile error |
| [`readonly.ds`](readonly.ds) | deep readonly views, mutation as a compile error |
| [`reflection.ds`](reflection.ds) | `Type<T>` and comptime layout queries |
