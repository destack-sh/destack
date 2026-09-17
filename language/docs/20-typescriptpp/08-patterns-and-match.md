---
title: Patterns and Match
description: Pattern matching.
---

# Patterns and Match

- patterns
- refutable vs irrefutable
- match
- `Sequence` type
- ... except for dynamic index signatures where it types as `V | undefined` (via dynamic.find)
- ranges: `..`
- switch still works but match encouraged

- let, let-else
- if let
- while let
- match guards

- No Computed Keys
- no `obj[expr]` where `expr` is dynamic
- destructure dynamically with `{ [key]: value }`?

```ds:src/match.ds
enum State { Pending, Running, Complete }
declare const state: State;

const label = match (state) {
    State.Pending => "pending",
    State.Running => "running",
    State.Complete => "complete",
};
```

- enums keep their type under every narrowing
- testing for `null` or `undefined` are just tag reads (no borrow needed)
