---
title: Objects
description: Fixed static shapes.
---

# Objects

```ds:src/objects.ds
const relay = {
    name: "zurich",
    active: true,
};

relay.name;
```

- what doees `type Point = { x: number; y: number }` mean?
- can I pass `{ x: 0, y: 1, z: 2 }` to a function expecting a `Point`?
- (no, has to match exactly, in order)
- but you can pass it to `Dynamic<Point>` or `interface Point` (same thing)

- deep readonly
- const is *not* readonly (just like in TS)
- const bindings are never reassigned
