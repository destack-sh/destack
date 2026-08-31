---
title: Interfaces
description: Structural interfaces are a key part of TypeScript.
---

# Interfaces

```ds:src/interfaces.ds
interface Named {
    name: string;
}

function label(value: Named): string {
    value.name
}
```
