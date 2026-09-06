---
title: Structs
description: Fixed no overhead shapes.
---

# Structs

- eventually, every serious programming language cares about memory layout and allocations
- need fixed no overhead shapes
- no inheritance
- no embedding (unlike Go, Jai)
- no constructors, getters or setters

- `this` = `&exclusive T` for value types (more on that soon)

```ds:src/point.ds
struct Point {
    x: float64;
    y: float64;
}

const origin = Point { x: 0.0, y: 0.0 };
```
