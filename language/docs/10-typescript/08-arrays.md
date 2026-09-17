---
title: Arrays
description: Arrays, proper tuples, slices, inline arrays and the rest.
---

# Arrays

- arrays, proper, tuples, slices, inline arrays and the rest
- first, arrays work as before
- T[] == Array<T>

```ds:src/collections.ds
const targets: string[] = ["web", "native"];
```

- `Array.sort` and `toSorted` follow TS string ordering by default, and their comparators return a number
- `sortUnstable` takes an `Ordering` comparator
- mutators such as `push`, `pop`, `sort`, and `retain` take `&this`; `take` needs `&exclusive this` or a `Copy` element
- `indexOf` returns `-1` when absent
- `Set` and `Map` keys compare by SameValueZero through `DefaultEqual`, which scalars implement
