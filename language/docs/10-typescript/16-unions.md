---
title: Unions
description: Nominal and structural discriminated unions.
---

# Unions

- sum types
- regular unions
- nominal and structural discriminated unions
- discriminated tags lower as simple types

```ds:src/events.ds
type Event =
    | { kind: "ready"; port: uint16 }
    | { kind: "closed"; reason: string };
```

- union member dispatch resolves every variant
- one common implementation stays static
- otherwise the compiler emits a runtime case dispatch and unions the result types
