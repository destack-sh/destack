---
title: Concurrency
description: Worker-first, local-first, shared-nothing-first memory model.
---

# Concurrency

```ds:src/worker.ds
import { spawn } from "destack:worker";

const worker = spawn((value: uint64) => value * 2);
const result = await worker.post(21);
```
