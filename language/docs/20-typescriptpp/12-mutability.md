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
- four rungs: `&readonly` < `&` / `&immutable` < `&exclusive`; immutability and exclusivity are guarantees only owned storage can establish
- (what about data races..? lints / DST / ...)
- unique ownership
- worker-local, borrowing

- WithAccess
- PlaceOf
- AccessOf, Local, Shared, ...

- `readonly T` is deep through fields, indexing, and references obtained through the view; a separate mutable alias can still modify the object (just like in TS, but unlike RusT!, we have `&immutable` for that)
- readonly preserves ownership, lifetimes, and copyability; a non-Copy owner stays non-Copy
- ordinary shared access is readonly; synchronized guards may lend mutable exclusive access and their borrows end before unlocking
- `WithAccess` changes access and preserves region and reference layers; `AccessOf` and `PlaceOf` inspect qualifications
