---
title: Structs and Value Types
description: Nominal value types with fixed layout.
---

# Structs and Value Types

```ds:src/geometry.ds
export struct Point {
    x: float64;
    y: float64;
}

export const origin = Point { x: 0.0, y: 0.0 };
```
