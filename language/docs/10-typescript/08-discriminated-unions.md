---
title: Discriminated Unions
description: Nominal and structural discriminated unions.
---

# Discriminated Unions

```ds:src/events.ds
type Event =
    | { kind: "ready"; port: uint16 }
    | { kind: "closed"; reason: string };
```
