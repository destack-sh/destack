---
title: Drop
description: Drop is for "infallible" memory management, using is for actual resources
---

# Drop

- Drop is *eager* (unlike in Rust)
- Drop is for "infallible" memory management, using is for actual resources
- Drop also runs as a "finaliser"
- (e.g. Drop on an Array deallocates the memory)
- drop flags in the frame cover conditional initialisation and partial moves, as in Rust drop elaboration
- Drop is *not* lowered to JS (not sure how that would even work..?)
- owned locals drop after their last use, owned fields drop with their parent, and managed allocations run `Drop` when reclaimed

```ds
export newtype interface Drop {
    /// Drop this value.
    drop(&this): void;
}
```

- Drop is a finalizer, yes, but a very restricted one
- no allocations, no panics, statically checked
- Drop is designed to only deal with memory deallocation: no allocation, panic, suspension, or resurrection; resources use `using` and `async using`

- moving one field out of a type with a user `Drop` is rejected; other aggregates clean up their remaining fields
- `drop(value)` ends ownership immediately
- `forget(value)` suppresses automatic drop
- `ManuallyDrop<T>` stores outside automatic drop
- and `Box<T>.leak()` yields a static borrow
