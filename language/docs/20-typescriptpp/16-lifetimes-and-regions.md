---
title: Lifetimes and Regions
description: Generalised lifetimes into regions.
---

# Lifetimes and Regions

```ds:src/lifetimes.ds
function first<'a, T>(values: &'a readonly T[]): &'a readonly T {
    &values[0]
}
```

- as soon as we pass and store references, we need to make sure those are safe too
- well wouldn't you know, lifetimes
- generalised lifetimes into regions (combine lifetime + space/place)
- T & 'a, 'a & "shared", ...
- Borrowed<T, L/R, A>, REadonlyBorrowed, ExclusiveBorrowed

- stored borrows write lifetime parameters explicitly; function signatures infer hidden lifetime parameters, prefer the receiver lifetime, union borrowed input lifetimes, and otherwise use `"static"`

- a borrow used to initialize a binding extends its temporary to the binding lifetime; other
temporaries live to the end of the enclosing statement

- a region is an extent and a place; access and exclusivity are independent, and joins keep sets of extent/place pairs
- a closure / coroutine carries the lifetime of every borrow it captures on its type, through callable and interface conversions alike
