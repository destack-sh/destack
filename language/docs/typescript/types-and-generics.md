---
title: Types and Generics
description: Inference, aliases, unions, generics, and constraints.
order: 107
---

# Types and Generics

Destack follows strict TypeScript inference, unions, and generics.

```ds:src/types.ds
type Identifier = string | uint64;
type Coordinates = readonly [float64, float64];

const relay = {
    id: "relay_zurich",
    position: [47.3769, 8.5417],
} satisfies {
    id: Identifier;
    position: Coordinates;
};

type Relay = typeof relay;
```
