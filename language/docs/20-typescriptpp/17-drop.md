---
title: Drop
description: Drop is for "infallible" memory management, using is for actual resources
---

# Drop

- Drop is *eager* (unlike in Rust)
- Drop is for "infallible" memory management, using is for actual resources
- Drop also runs as a "finaliser"
- (e.g. Drop on an Array deallocates the memory)
- no drop flags needed because no partial initialisation + eager drop
- Drop is *not* lowered to JS (not sure how that would even work..?)
- owned locals drop after their last use, owned fields drop with their parent, and managed allocations run `Drop` when reclaimed

```ds
export newtype interface Drop {
    /// Drop this value.
    drop(&exclusive this): void;
}
```

- Drop is a finalizer, yes, but a very restricted one
- no allocations, no panics, statically checked

- no drop flags
- maybe-present values use explicit unions; conditional moves are rejected at control-flow joins, so runtime drop flags are unnecessary
- `drop(value)` ends ownership immediately
- `forget(value)` suppresses automatic drop
- `ManuallyDrop<T>` stores outside automatic drop
- and `Box<T>.leak()` yields a static borrow
