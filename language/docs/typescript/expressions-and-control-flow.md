---
title: Expressions and Control Flow
description: Expressions, conditions, loops, and matches.
order: 104
---

# Expressions and Control Flow

Blocks, conditions, loops, and matches produce values. Their final expression is the result.

```ds:src/expressions.ds
type Status = "ready" | "waiting" | "failed";

export function label(status: Status): string {
    match (status) {
        "ready" => "Ready";
        "waiting" => "Waiting";
        "failed" => "Failed";
    }
}

export const port = do {
    const base = 8000;
    base + 80
};
```

```ds:src/control.ds
import { log } from "destack:console";

export function report(values: int32[]): void {
    for (const value of values) {
        const label = match (value) {
            0 => "zero";
            value if (value % 2 == 0) => "even";
            _ => "odd";
        };

        log(`${value}: ${label}`);
    }
}
```
