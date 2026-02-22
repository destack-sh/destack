# Newtypes

Newtypes are nominal wrappers that prevent mixing semantically different values.
These tests cover constructors, assignability boundaries, and module forwarding.

## Coverage

- `newtype.md`: Constructor behavior and baseline nominal identity.
- `assignability.md`: Explicit assignability boundaries against backing types and sibling newtypes.
- `modules.md`: Cross-module newtype identity and re-export forwarding.
