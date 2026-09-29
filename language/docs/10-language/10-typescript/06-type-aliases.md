---
title: Type Aliases
description: Type algebra.
---

# Type Aliases

```tspp src/aliases.tspp
type Identifier = string | uint64;
type Position = readonly (float64, float64);
```

## Satisfies

```tspp
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
```
