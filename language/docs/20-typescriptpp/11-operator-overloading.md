---
title: Operator Overloading
description: Serious math-y applications want operator overloading.
---

# Operator Overloading

```ds:src/vector.ds
extension of Vector2 implements Add<Vector2> {
    add(this, other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}
```
