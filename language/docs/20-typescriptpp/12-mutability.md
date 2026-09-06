---
title: Mutability
description: let / const preserve TS meaning
---

# Mutability

- let / const preserve TS meaning
- const does *not* imply deep readonly
- "as const" _is_ deep readonly
- can take &exclusive only on managed types for const
- readonly, readonly modifier, readonly T

- &T default to mutable
- &readonly for explicit readonly
- Rust only has mutable vs immutable
- "third rung" on the mutability ladder
- overwrite stability
- we can now distinguish "readonly, non-exclusive", "mutable, non-exclusive", "mutable, exclusive"
- (what about data races..? lints / DST / ...)
- exclusive ownership
- worker-local, borrowing

- WithAccess
- PlaceOf
- AccessOf, Local, Shared, ...
