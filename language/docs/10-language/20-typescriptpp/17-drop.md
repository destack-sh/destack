---
title: Drop
description: Drop is for "infallible" memory management, using is for actual resources
---

# Drop

- Drop runs at the end of ownership, with drop flags for conditional initialisation, as in Rust drop elaboration
- Drop is for "infallible" memory management, using is for actual resources
- Drop also runs as a "finaliser"
- (e.g. Drop on an Array deallocates the memory)
- drop flags in the frame cover conditional initialisation and partial moves
- Drop is *not* lowered to JS (not sure how that would even work..?)
- an owned local drops after its last use since `Drop` is memory only, an owned field drops with its parent, and a managed allocation runs `Drop` when reclaimed

```tspp
export newtype interface Drop {
    /// Drop this value.
    drop(&exclusive this): void;
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

- destructors get `&exclusive this`: the owner runs it at scope end, overwrite, or explicit drop, and the collector runs it on an unreachable block, so nothing else reaches the value
- a uniquely owned allocation whose values moved out frees immediately (unless heap storage retained it)
- a retained one is marked empty, and the collector frees the storage on its next run without running any drop
- a consumed once-closure environment releases its storage the same way after its captures moved out
