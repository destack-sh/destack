---
title: Structs
description: Fixed no overhead shapes.
---

# Structs

Every serious programming language cares about memory layout and allocations.
TS++ adds `struct`s for "zero overhead" data shapes (or "plex") without any headers, vtables, inheritance, getters or setters.
Unlike in Go or Jai, TS++ structs have no "embedding", just regular composition.

```ds:src/point.ds
struct Point {
    x: float64;
    y: float64;
}

let origin = Point { x: 1.0, y: 1.0 };
let pivot = origin;

console.log(origin.x); // 1.0
console.log(pivot.x); // 1.0;

origin.x = 0.0;

console.log(origin.x); // 0.0;
console.log(pivot.x); // 1.0; // pivot is a copy of origin
```
