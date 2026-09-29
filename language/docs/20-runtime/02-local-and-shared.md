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
- `shared class` places a class and every handle to it in the shared heap; a type expression carries no space
- `shared` marks a binding or a declaration; a plain one is local
- worker-local stuff is .. local (Promise, Task, etc.)
- no need for Send and Sync, basically the 90 degree rotated version of that classic pair

- local borrowing of managed storage is sound, across suspension too: frames live in world memory and borrows are checked across calls and parks
- how to keep local / shared safe
- proper managed object types on shared

- a borrow takes the space of its referent; one body serves every space
- reference types are local by default unless declared shared
- `SharedSafe` decides what shared storage holds and what crosses workers
- the constant image is read from every worker and the collector skips it by address; no type names it
- ordinary shared access is readonly; interior mutation types encapsulate synchronized access, and a guard's borrows end before it unlocks
- shared publication must not retain unsynchronized writable aliases
