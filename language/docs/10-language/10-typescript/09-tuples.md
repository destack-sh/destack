---
title: Tuples
description: Tuples
---

# Tuples

- no more array tuples (need to free up `[T]` and `[T; N]`, arbitrary `[X, Y, ...]` is an error)
- proper tuples! (A, B, C)
- empty tuple == `() == void`

```tspp src/collections.tspp
const position: (float64, float64) = (47.3769, 8.5417);
```
