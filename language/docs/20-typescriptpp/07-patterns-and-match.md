---
title: Patterns and Match
description: Pattern matching.
---

# Patterns and Match

```ds:src/match.ds
const label = match (state) {
    State.Pending => "pending",
    State.Running => "running",
    State.Complete => "complete",
};
```
