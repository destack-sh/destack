---
title: Narrowing
description: Type narrowing as usual.
---

# Narrowing

- type narrowing as usual, narrowing is just doing runtime type checking
- instanceof, typeof, is
- typeof in type position
- is for type queries
- instanceof for classes
- match narrowing

```ds:src/narrowing.ds
type Event =
    | { kind: "ready"; port: uint16 }
    | { kind: "closed"; reason: string };

function describe(event: Event): string {
    if (event.kind == "ready") {
        return `listening on ${event.port}`;
    }

    event.reason
}
```
