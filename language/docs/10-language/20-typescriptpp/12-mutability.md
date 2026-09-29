---
title: Mutability
description: let / const preserve TS meaning
---

# Mutability

- let / const preserve TS meaning
- const does *not* imply deep readonly
- "as const" _is_ deep readonly
- readonly, readonly modifier, readonly T

## Access

- &T default to mutable
- &readonly for explicit readonly
- Rust only has mutable vs immutable
- four 2x2 rungs: `&readonly` < `&` / `&immutable` < `&exclusive`

- immutability and exclusivity are guarantees owned storage establishes, and a handle admits them only where no alias could invalidate the bits (see borrowing)

- (what about data races..? lints / DST / ...)
- unique ownership
- worker-local, borrowing

- `WithAccess` changes access and preserves region and reference layers; `AccessOf` inspects the access a qualified type grants

## Readonly

- `readonly T` is deep through owned structure (fields, indexing, borrows obtained through the view) and stops at a managed handle
- `readonly Array<readonly Node>` or `DeepReadonly<T>` is _explicitly_ deep through handles
- a separate mutable alias can still modify the object (just like in TS, but unlike Rust, we have `&immutable` for that)
- readonly preserves ownership, lifetimes, and copyability; a non-Copy owner stays non-Copy
- ordinary shared access is readonly; synchronized guards may lend mutable exclusive access and their borrows end before unlocking
