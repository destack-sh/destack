---
title: Operator Overloading
description: Serious math-y applications want operator overloading.
---

# Operator Overloading

- operator overloading
- serious math-y applications want operator overloading
- newtype interfaces ("traits")
- binary `Add`, `Subtract`, `Multiply`, `Divide`, etc.
- unary `Plus`, `Negate`
- operators dispatch through standard interfaces on the left operand only; the reverse operand order needs its own implementation

```ds:src/vector.ds
import { Add } from "destack:ops";

struct Vector2 {
    x: float64;
    y: float64;
}

extension of Vector2 implements Add<Vector2> {
    type Output = Vector2;

    add(this, other: Vector2): Vector2 {
        Vector2 { x: this.x + other.x, y: this.y + other.y }
    }
}
```
