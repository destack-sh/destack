---
title: Slices and Fixed Arrays
description: Slices and Fixed Arrays
---

# Slices and Fixed Arrays

- fixed arrays `[T; N]`

- slices are just fat pointers
- `&[T]` is also a fat pointer, `[T]` is a managed slice, `^[T]` is an owned slice
- `[T]` is managed by default (just like `Function` and `Dynamic` are managed fat pointers by default)

- subslicing: a range subscript is a view, not a copy — `values[1..4]` borrows a window of the backing store
- the view carries the receiver's access: a managed receiver gives a managed `[T]` view, `&readonly`/`&` give same-access borrowed views
- `.slice()` keeps its TypeScript meaning and copies; `.toOwned()` copies a view into owned storage
- `values[1..4] = other` copies in — lengths must match, elements must be `Copy`, overlapping ranges copy like `memmove`
- `splitAt` through `&exclusive` yields two disjoint exclusive views
- a view keeps its backing array alive; owned slices `^[T]` move as a whole and never split
- a view keeps its region and can only weaken access or exclusivity
