---
title: Tasks
description: Tasks
---

# Tasks

- introduce Task for structured affine concurrency (same async/await model)
- (Promise = managed class, Task = value type, Promise requires aliasable / copyable type)
- `Task<T>` is consuming Worker-local completion with cancellation and scope ownership

- every started operation must be awaited, returned, or handed to a scope; `no-floating-promises` is denied by default
