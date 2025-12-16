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
| **Inherent** | `extension for T { }` (same file as T) | Automatic wherever T is used |
| **Local** | `extension for T { }` (T from another file) | Only in defining file |
| **Named** | `extension Name for T { }` | Must be imported to use (or local) |

## Not Yet Covered

- Operator overloading via `implements Add<T>` etc. (not yet implemented)
- Named extension imports across modules (not yet implemented)
