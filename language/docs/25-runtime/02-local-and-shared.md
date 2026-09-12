---
title: Local and Shared
description: local isolated heap per worker
---

# Local and Shared

- NOTE: maybe move local / shared into typescrippp..? not sure.
- so far we have assumed basically single-threaded, async execution
- this is most code, but obviously a complete language needs to consider concurrency at a more fundamental level, across threads
- many ways to do this, TS already strongly biases into the "local-first" direction
- we could just generalise SharedArrayBuffer and friends?
- split local and shared memory spaces
- separate heaps, separate GCs
- worker-first, local-first, shared-nothing-first memory model

- local isolated heap per worker
- local and shared modifier on types
- local and shared modifier on bindings
- local and shared modifier on declarations
- worker-local stuff is .. local (Promise, Task, etc.)
- no need for Send and Sync, basically the 90 degree rotated version of that classic pair

- local borrowing of managed storage is sound, across suspension too: frames live in world memory and borrows are checked across calls and parks
- how to keep local / shared safe
- proper managed object types on shared

- borrows are place polymorphic by default
- reference types are local by default unless otherwise specified
- `SharedSafe`
- ordinary shared access is readonly; interior mutation types encapsulate synchronized access, and a guard's borrows end before it unlocks
- shared publication must not retain unsynchronized writable aliases
