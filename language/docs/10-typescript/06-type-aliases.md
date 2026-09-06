---
title: Type Aliases
description: Type algebra.
---

# Type Aliases

```ds:src/aliases.ds
type Identifier = string | uint64;
type Position = readonly (float64, float64);
```

## Satisfies

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
```
