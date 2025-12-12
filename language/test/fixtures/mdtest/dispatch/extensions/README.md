# Extensions

Extensions add methods to existing types without modifying them.

## Specification

From [SPECIFICATION.md](../../../../../SPECIFICATION.md#extension):

- Extensions can add methods to any type: structs, classes, interfaces, enums, newtypes, primitives
- Extension visibility depends on where the extension is defined relative to the type
- Extensions can implement interfaces (for operator overloading, not yet supported)
- Extensions can be generic

## Extension Kinds

| Kind | Syntax | Visibility |
|------|--------|------------|
| **Inherent** | `extension T { }` (same file as T) | Automatic wherever T is used |
| **Local** | `extension T { }` (T from another file) | Only in defining file |
| **Named** | `extension Name: T { }` | Must be imported to use |

## Files

| File | Description |
|------|-------------|
| `basic.md` | Basic extension declarations on structs, classes, interfaces, enums |
| `methods.md` | Extension method signatures: parameters, returns, `this`, static members |
| `visibility.md` | Extension visibility rules for inherent, local, and named extensions |
| `generics.md` | Generic extensions with type parameters |
| `implements.md` | Extensions implementing interfaces |
| `errors.md` | Error cases: missing methods, type mismatches, visibility errors |

## Not Yet Covered

- Operator overloading via `implements Add<T>` etc. (not yet implemented)
- Named extension imports across modules (not yet implemented)
