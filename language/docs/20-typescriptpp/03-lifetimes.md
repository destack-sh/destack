---
title: Lifetimes
description: Generalised lifetimes into regions.
---

# Lifetimes

```ds:src/lifetimes.ds
function first<'a, T>(values: &'a readonly T[]): &'a readonly T {
    &values[0]
}
```
