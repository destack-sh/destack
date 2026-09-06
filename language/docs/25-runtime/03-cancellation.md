---
title: Cancellation
description: Cancellation
---

# Cancellation

- `TaskScope` owns work that outlives a frame; asynchronous disposal cancels pending children and waits for cleanup
- cancellation resumes a parked continuation into its cancellation path, runs `using`, `finally`, and `Drop`, and stops at the task boundary

- `Promise<T>` is repeatable Worker-local completion for copyable values
