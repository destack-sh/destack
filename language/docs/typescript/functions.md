---
title: Functions
description: Functions, lambdas, generics, and captures.
order: 105
---

# Functions

Functions use TypeScript signatures. Lambdas may state how their environment is captured.

```ds:src/functions.ds
export function identity<T>(value: T): T {
    value
}

export function multiplier(factor: int32): (int32) => int32 {
    @capture({ factor: "copy" })
    return (value) => value * factor;
}

export const double = multiplier(2);
```
