---
title: Modules
description: Strictly ESM imports and exports.
---

# Modules

```ds:src/geometry.ds
export function square(value: float64): float64 {
    value * value
}
```

```ds:src/main.ds
import { square } from "./geometry.ds";

square(4.0);
```
