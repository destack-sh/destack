---
title: Workers
description: Worker-first, local-first, shared-nothing-first memory model.
---

# Workers

- JS/TS already has a strong worker-first story
- already talked about local / shared heap split
- shared memory across workers, local heap to each worker
- (this is fortunate because it gives us the local managed/borrowed model we want)
- retain local / worker isolation as the primary model
- use Workers for structured concurrency
- (maps to threads N:M)

```ds:src/worker.ds
import { spawn } from "destack:worker";

const worker = spawn((value: uint64) => value * 2);
const result = await worker.post(21);
```
