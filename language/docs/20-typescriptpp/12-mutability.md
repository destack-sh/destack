---
title: Mutability
description: let / const preserve TS meaning
---

# Mutability

- let / const preserve TS meaning
- const does *not* imply deep readonly
- "as const" _is_ deep readonly
- readonly, readonly modifier, readonly T

- &T default to mutable
- &readonly for explicit readonly
- Rust only has mutable vs immutable
- two rungs: readonly and mutable; exclusivity is a property of owned storage, not of the reference
- (what about data races..? lints / DST / ...)
- unique ownership
- worker-local, borrowing

- WithAccess
- PlaceOf
- AccessOf, Local, Shared, ...
