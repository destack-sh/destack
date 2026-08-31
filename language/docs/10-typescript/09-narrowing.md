---
title: Narrowing
description: Type narrowing as usual.
---

# Narrowing

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
