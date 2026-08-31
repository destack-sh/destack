---
title: Constraints
description: Where clauses.
---

# Constraints

```ds:src/constraints.ds
function collect<T, C>(values: T[]): C where C: FromIterator<T> {
    values.intoIterator().collect()
}
```
