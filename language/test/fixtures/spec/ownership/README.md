# Ownership (LanguageFeature::Ownership)

> NOTE #Incomplete: implement/mdtest ownership semantics

Explicit control over references, values, and mutability.

TypeScript doesn't distinguish references from values. Destack adds opt-in
explicit control.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `references/` | Reference types (`&T`, `&var T`) |
| `values/` | Value types (`^T`, `^var T`) |
| `mutability/` | Mutability modifiers (`const`, `var`) |

## Overview

### Value Ownership

```ds
T            // automatic (TypeScript behavior)
&T           // reference (shared access)
^T           // value (copy semantics)
```

### Mutability

```ds
&const T     // immutable reference
&var T       // mutable reference
^const T     // immutable value
^var T       // mutable value
```

### Dispatch Behavior

```ds
&T             // automatic dispatch
&implements T  // explicit dynamic interface dispatch
&extends T     // explicit dynamic class dispatch
```

See [DESIGN.md](../../../../../DESIGN.md#ownership) for full documentation.
