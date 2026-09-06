---
title: Enums
description: A known set of allowed values.
---

# Enums

- enums are reasonably simple, they're just a known set of allowed values
- and are represent by their exact value
- no const enum needed?
- just integer and string enums..?
- auto incrementing enum (starts at 0, int64, signed)
- enums are nominal! need to explicitly cast

- enums are carried by their value, unlike unions / literals
- so int enums are actual ints, string enums are just managed strings

- like all nominal types enums may carry instance members, constants, etc.

```ds:src/state.ds
enum State {
    Pending,
    Running,
    Complete,
}
```
