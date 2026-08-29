---
title: Modules
description: Static imports and exports.
---

# Modules

Every file is an ESM module. Local modules and standard-library packages use the same static
import graph.

```ds:src/geometry.ds
export struct Point {
    x: float64;
    y: float64;
}

export function distance(point: Point): float64 {
    (point.x * point.x + point.y * point.y).sqrt()
}
```

```ds:src/main.ds
import { log } from "destack:console";

import { Point, distance } from "./geometry.ds";

export function main(): void {
    const point = Point { x: 3.0, y: 4.0 };
    log(distance(point));
}
```
